import io
import os
import logging
import torch
import requests
from functools import lru_cache
from flask import Flask, request, jsonify
from PIL import Image, UnidentifiedImageError
from transformers import CLIPProcessor, CLIPModel, CLIPTokenizer
from dotenv import load_dotenv

load_dotenv()

logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')

MAX_FILE_SIZE_MB = int(os.getenv("MAX_FILE_SIZE_MB", 5))
MAX_TEXT_LENGTH = int(os.getenv("MAX_TEXT_LENGTH", 2000))
MAX_FILES = int(os.getenv("MAX_FILES", 1))
PORT = int(os.getenv("PORT", 8000))
MAX_CONTEXT_LENGTH = 77
CHUNK_SIZE = 64
SLIDING_WINDOW_STEP = 32

device = "cuda" if torch.cuda.is_available() else "cpu"

try:
    logging.info("Loading CLIP model...")
    logging.info(f"Device: {device}")
    model = CLIPModel.from_pretrained("openai/clip-vit-large-patch14").to(device)
    processor = CLIPProcessor.from_pretrained("openai/clip-vit-large-patch14")
    tokenizer = CLIPTokenizer.from_pretrained("openai/clip-vit-large-patch14")
    logging.info("CLIP model loaded successfully")
except Exception as e:
    logging.error(f"Error loading model: {e}")
    model = None
    processor = None
    tokenizer = None

app = Flask(__name__)

@lru_cache(maxsize=128)
def _encode_text_cached(text):
    return tokenizer.encode(text, add_special_tokens=False)

def download_image_from_url(url):
    try:
        logging.info(f"Downloading image from: {url[:50]}...")
        response = requests.get(url, timeout=10, stream=True)
        response.raise_for_status()
        
        image = Image.open(io.BytesIO(response.content)).convert("RGB")
        logging.info(f"Image downloaded successfully, size: {image.size}")
        return image
    except Exception as e:
        logging.error(f"Error downloading image: {e}")
        raise ValueError(f"Failed to download image from URL: {e}")

def calculate_sliding_window_chunks(token_count, chunk_size=CHUNK_SIZE, step_size=SLIDING_WINDOW_STEP):
    if token_count <= chunk_size:
        return 1
    
    num_chunks = 0
    for i in range(0, token_count, step_size):
        chunk_tokens = min(chunk_size, token_count - i)
        if chunk_tokens < chunk_size // 2:
            break
        num_chunks += 1
        if i + chunk_size >= token_count:
            break
    return max(1, num_chunks)

def chunk_text_tokens_sliding_window(text, chunk_size=CHUNK_SIZE, step_size=SLIDING_WINDOW_STEP):
    if tokenizer is None:
        raise ValueError("Tokenizer not available")
    
    tokens = _encode_text_cached(text)
    if len(tokens) <= chunk_size:
        return [text]
    
    chunks = []
    for i in range(0, len(tokens), step_size):
        chunk_tokens = tokens[i:i + chunk_size]
        if len(chunk_tokens) < chunk_size // 2:
            break
        chunk_text = tokenizer.decode(chunk_tokens, skip_special_tokens=True)
        chunks.append(chunk_text)
        
        if i + chunk_size >= len(tokens):
            break
    
    if len(chunks) > 1:
        logging.info(f"Text split into {len(chunks)} overlapping chunks using sliding window")
    return chunks

def process_text_chunks(text):
    chunks = chunk_text_tokens_sliding_window(text)
    if len(chunks) == 1:
        inputs = processor(text=[text], return_tensors="pt", padding=True).to(device)
        with torch.no_grad():
            text_features = model.get_text_features(**inputs)
            return text_features / text_features.norm(dim=-1, keepdim=True)
    
    chunk_features = []
    for chunk in chunks:
        inputs = processor(text=[chunk], return_tensors="pt", padding=True).to(device)
        with torch.no_grad():
            features = model.get_text_features(**inputs)
            chunk_features.append(features / features.norm(dim=-1, keepdim=True))
    
    return torch.stack(chunk_features).mean(dim=0)

def validate_model_availability():
    if model is None or processor is None or tokenizer is None:
        return jsonify({"error": "Model is not available. Please check the server logs."}), 503
    return None

def validate_request_size():
    if request.content_length > MAX_FILE_SIZE_MB * 1024 * 1024:
        return jsonify({"error": f"Request size exceeds {MAX_FILE_SIZE_MB}MB"}), 400
    return None

@app.route("/")
def home():
    return jsonify({"message": "CLIP ViT-L/14 Embedding API running!"})

@app.route("/embed/text", methods=["POST"])
def embed_text():
    logging.info("Processing text embedding request")
    
    size_error = validate_request_size()
    if size_error:
        return size_error
        
    data = request.get_json()
    if not data or "text" not in data:
        return jsonify({"error": "Invalid input. 'text' is required."}), 400
        
    text = data.get("text", "")
    if len(text) > MAX_TEXT_LENGTH:
        return jsonify({"error": f"Text length exceeds {MAX_TEXT_LENGTH} characters"}), 400
        
    model_error = validate_model_availability()
    if model_error:
        return model_error
        
    try:
        text_features = process_text_chunks(text)
        embedding = text_features.cpu().numpy().flatten().tolist()
        
        tokens = _encode_text_cached(text)
        was_chunked = len(tokens) > CHUNK_SIZE
        num_chunks = calculate_sliding_window_chunks(len(tokens)) if was_chunked else 1
        
        metadata = {
            "chunked": was_chunked,
            "num_chunks": num_chunks,
            "original_token_count": len(tokens),
            "chunk_method": "sliding_window" if was_chunked else "single"
        }
        
        logging.info(f"Text embedding generated successfully, chunks: {num_chunks}")
        return jsonify({
            "embedding": embedding,
            "metadata": metadata
        })
    except Exception as e:
        logging.error(f"Error processing text: {e}")
        return jsonify({"error": "An error occurred while processing the text."}), 500

@app.route("/embed/image", methods=["POST"])
def embed_image():
    logging.info("Processing image embedding request")
    
    size_error = validate_request_size()
    if size_error:
        return size_error
        
    model_error = validate_model_availability()
    if model_error:
        return model_error
        
    try:
        images = []
        
        if request.is_json:
            data = request.get_json()
            if "url" in data:
                images.append(download_image_from_url(data["url"]))
            elif "image_url" in data:
                images.append(download_image_from_url(data["image_url"]))
            elif "image_urls" in data and data["image_urls"]:
                image_urls = data["image_urls"] if isinstance(data["image_urls"], list) else [data["image_urls"]]
                for url in image_urls[:MAX_FILES]:
                    images.append(download_image_from_url(url))
                    
        if not images and len(request.files) > 0:
            if len(request.files) > MAX_FILES:
                return jsonify({"error": f"Too many files uploaded. Maximum allowed is {MAX_FILES}"}), 400
                
            for file in request.files.values():
                image = Image.open(file.stream).convert("RGB")
                images.append(image)
            
        if not images and request.form.get("url"):
            images.append(download_image_from_url(request.form.get("url")))
            
        if not images:
            return jsonify({"error": "No image provided. Send either a file upload or URL(s)."}), 400
            
        logging.info(f"Processing {len(images)} image(s)")
        
        if len(images) == 1:
            image = images[0]
            inputs = processor(images=image, return_tensors="pt").to(device)
            
            with torch.no_grad():
                image_features = model.get_image_features(**inputs)
                image_features = image_features / image_features.norm(dim=-1, keepdim=True)
                embedding = image_features.cpu().numpy().flatten().tolist()
        else:
            all_features = []
            
            for image in images:
                inputs = processor(images=image, return_tensors="pt").to(device)
                
                with torch.no_grad():
                    image_features = model.get_image_features(**inputs)
                    image_features = image_features / image_features.norm(dim=-1, keepdim=True)
                    all_features.append(image_features)
            
            averaged_features = torch.stack(all_features).mean(dim=0)
            embedding = averaged_features.cpu().numpy().flatten().tolist()
            
        metadata = {
            "num_images": len(images),
            "processing_method": "single" if len(images) == 1 else "averaged"
        }
        
        logging.info(f"Image embedding generated successfully, images: {len(images)}")
        return jsonify({
            "embedding": embedding,
            "metadata": metadata
        })
        
    except UnidentifiedImageError as e:
        logging.error(f"Invalid image format: {e}")
        return jsonify({"error": "Invalid image format."}), 400
    except ValueError as e:
        logging.error(f"ValueError: {e}")
        return jsonify({"error": str(e)}), 400
    except Exception as e:
        logging.error(f"Error processing image: {e}")
        return jsonify({"error": "An error occurred while processing the image."}), 500

@app.route("/embed/combined", methods=["POST"])
def embed_combined():
    logging.info("Processing combined text-image embedding request")
    
    size_error = validate_request_size()
    if size_error:
        return size_error
        
    has_image = len(request.files) > 0
    has_text = 'text' in request.form or (request.is_json and 'text' in request.get_json())
    has_image_url = False
    
    if request.is_json:
        data = request.get_json()
        has_image_url = "image_url" in data or "url" in data or "image_urls" in data
    else:
        has_image_url = "image_url" in request.form or "url" in request.form
            
    if not has_text:
        return jsonify({"error": "Text is required for combined embedding"}), 400
        
    model_error = validate_model_availability()
    if model_error:
        return model_error
        
    try:
        if request.is_json:
            data = request.get_json()
            text = data.get("text", "")
        else:
            text = request.form.get("text", "")
            
        if len(text) > MAX_TEXT_LENGTH:
            return jsonify({"error": f"Text length exceeds {MAX_TEXT_LENGTH} characters"}), 400
            
        if has_image or has_image_url:
            logging.info("Processing image component")
            images = []
            
            if has_image:
                for file in request.files.values():
                    image = Image.open(file.stream).convert("RGB")
                    images.append(image)
            else:
                if request.is_json:
                    data = request.get_json()
                    if "image_urls" in data and data["image_urls"]:
                        image_urls = data["image_urls"] if isinstance(data["image_urls"], list) else [data["image_urls"]]
                        for url in image_urls[:MAX_FILES]:
                            images.append(download_image_from_url(url))
                    else:
                        image_url = data.get("image_url") or data.get("url")
                        if image_url:
                            images.append(download_image_from_url(image_url))
                else:
                    image_url = request.form.get("image_url") or request.form.get("url")
                    if image_url:
                        images.append(download_image_from_url(image_url))
                
            if not images:
                return jsonify({"error": "No valid images provided"}), 400
                
            text_features = process_text_chunks(text)
            
            if len(images) == 1:
                image_inputs = processor(images=images[0], return_tensors="pt").to(device)
                with torch.no_grad():
                    image_features = model.get_image_features(**image_inputs)
                    image_features = image_features / image_features.norm(dim=-1, keepdim=True)
            else:
                all_image_features = []
                for image in images:
                    image_inputs = processor(images=image, return_tensors="pt").to(device)
                    with torch.no_grad():
                        img_feat = model.get_image_features(**image_inputs)
                        img_feat = img_feat / img_feat.norm(dim=-1, keepdim=True)
                        all_image_features.append(img_feat)
                
                image_features = torch.stack(all_image_features).mean(dim=0)
                
            combined_features = (image_features + text_features) / 2
            embedding = combined_features.cpu().numpy().flatten().tolist()
            
            tokens = _encode_text_cached(text)
            was_chunked = len(tokens) > CHUNK_SIZE
            num_chunks = calculate_sliding_window_chunks(len(tokens)) if was_chunked else 1
            metadata = {
                "text_chunked": was_chunked,
                "num_text_chunks": num_chunks,
                "num_images": len(images),
                "combination_method": "image_text_average",
                "chunk_method": "sliding_window" if was_chunked else "single"
            }
        else:
            text_features = process_text_chunks(text)
            embedding = text_features.cpu().numpy().flatten().tolist()
            
            tokens = _encode_text_cached(text)
            was_chunked = len(tokens) > CHUNK_SIZE
            num_chunks = calculate_sliding_window_chunks(len(tokens)) if was_chunked else 1
            metadata = {
                "text_chunked": was_chunked,
                "num_text_chunks": num_chunks,
                "combination_method": "text_only",
                "chunk_method": "sliding_window" if was_chunked else "single"
            }
            
        logging.info("Combined embedding generated successfully")
        return jsonify({
            "embedding": embedding,
            "metadata": metadata
        })
        
    except UnidentifiedImageError as e:
        logging.error(f"Invalid image format: {e}")
        return jsonify({"error": "Invalid image format."}), 400
    except ValueError as e:
        logging.error(f"ValueError: {e}")
        return jsonify({"error": str(e)}), 400
    except Exception as e:
        logging.error(f"Error processing combined input: {e}")
        return jsonify({"error": "An error occurred while processing the combined input."}), 500

if __name__ == "__main__":
    app.run(host="0.0.0.0", port=PORT)