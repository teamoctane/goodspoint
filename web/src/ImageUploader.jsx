import { useRef, useState } from 'react';
import { T } from './i18n';
import './App.css';

export default function ImageUploader({ 
  onImagesSelected, 
  maxImages = 2, 
  maxSizeMB = 5,
  accept = "image/*",
  className = "",
  disabled = false 
}) {
  const fileInputRef = useRef(null);
  const [selectedImages, setSelectedImages] = useState([]);
  const [isDragging, setIsDragging] = useState(false);
  const [error, setError] = useState('');

  // Handle file input change
  const handleFileChange = (e) => {
    const files = Array.from(e.target.files);
    handleFiles(files);
    // Clear the input so the same files can be selected again if needed
    e.target.value = '';
  };

  // Handle drag and drop
  const handleDragOver = (e) => {
    e.preventDefault();
    if (!disabled) {
      setIsDragging(true);
    }
  };

  const handleDragLeave = (e) => {
    e.preventDefault();
    setIsDragging(false);
  };

  const handleDrop = (e) => {
    e.preventDefault();
    setIsDragging(false);
    if (!disabled) {
      const files = Array.from(e.dataTransfer.files);
      handleFiles(files);
    }
  };

  // Process uploaded files
  const handleFiles = (files) => {
    setError('');
    
    // Filter only image files
    const imageFiles = files.filter(file => file.type.startsWith('image/'));
    
    if (imageFiles.length === 0) {
      setError('Please select valid image files');
      return;
    }

    // Combine with existing images, but limit total to maxImages
    const totalImages = selectedImages.length + imageFiles.length;
    let filesToAdd = imageFiles;
    
    if (totalImages > maxImages) {
      const remainingSlots = maxImages - selectedImages.length;
      if (remainingSlots <= 0) {
        setError(`Maximum ${maxImages} images allowed. Please remove existing images first.`);
        return;
      }
      filesToAdd = imageFiles.slice(0, remainingSlots);
      setError(`Only ${remainingSlots} more image${remainingSlots > 1 ? 's' : ''} can be added (maximum ${maxImages} total)`);
    }

    // Check file size
    const maxSizeBytes = maxSizeMB * 1024 * 1024;
    const oversizedFiles = filesToAdd.filter(file => file.size > maxSizeBytes);
    if (oversizedFiles.length > 0) {
      setError(`Each image must be less than ${maxSizeMB}MB`);
      return;
    }

    // Create preview URLs for new images
    const newImageData = filesToAdd.map(file => ({
      file,
      url: URL.createObjectURL(file),
      name: file.name
    }));

    // Add to existing images
    const updatedImages = [...selectedImages, ...newImageData];
    setSelectedImages(updatedImages);
    
    // Notify parent component
    if (onImagesSelected) {
      onImagesSelected(updatedImages);
    }
  };

  // Remove image
  const removeImage = (index) => {
    const newImages = [...selectedImages];
    URL.revokeObjectURL(newImages[index].url);
    newImages.splice(index, 1);
    setSelectedImages(newImages);
    
    // Notify parent component
    if (onImagesSelected) {
      onImagesSelected(newImages);
    }
  };

  // Clear all images
  const clearAllImages = () => {
    selectedImages.forEach(img => URL.revokeObjectURL(img.url));
    setSelectedImages([]);
    if (onImagesSelected) {
      onImagesSelected([]);
    }
  };

  return (
    <div className={`image-uploader ${className}`}>
      {error && (
        <div className="error-message">
          {error}
        </div>
      )}

      <div 
        className={`image-drop-zone ${isDragging ? 'dragging' : ''} ${disabled ? 'disabled' : ''}`}
        onDragOver={handleDragOver}
        onDragLeave={handleDragLeave}
        onDrop={handleDrop}
        onClick={() => !disabled && fileInputRef.current?.click()}
      >
        <div className="drop-zone-content">
          <span className="material-symbols-outlined">cloud_upload</span>
          <p>{T('drag_drop_images')}</p>
          <p className="file-limits">{T('max_2_images')}</p>
          <input
            ref={fileInputRef}
            type="file"
            accept={accept}
            multiple
            onChange={handleFileChange}
            className="file-input"
            disabled={disabled}
          />
        </div>
      </div>

      {selectedImages.length > 0 && (
        <div className="selected-images">
          <div className="selected-images-header">
            <h3>{T('selected_images')} ({selectedImages.length}/{maxImages})</h3>
            <button 
              className="btn-link clear-all-btn"
              onClick={clearAllImages}
              disabled={disabled}
            >
              Clear All
            </button>
          </div>
          <div className="image-preview-grid">
            {selectedImages.map((img, index) => (
              <div key={index} className="image-preview">
                <img src={img.url} alt={`Upload ${index + 1}`} />
                <div className="image-info">
                  <span className="image-name">{img.name}</span>
                  <span className="image-size">
                    {(img.file.size / 1024 / 1024).toFixed(2)} MB
                  </span>
                </div>
                <button 
                  className="remove-image-btn"
                  onClick={() => removeImage(index)}
                  disabled={disabled}
                >
                  <span className="material-symbols-outlined">close</span>
                </button>
              </div>
            ))}
          </div>
          {selectedImages.length < maxImages && !disabled && (
            <div className="add-more-images">
              <button 
                className="btn secondary"
                onClick={() => fileInputRef.current?.click()}
              >
                Add More Images ({maxImages - selectedImages.length} remaining)
              </button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
