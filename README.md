# GoodsPoint

A product search and recommendation platform built for Prosus. This application leverages Groq and indirectly utilizes LLAMA through Tavily integration for intelligent query processing and product recommendations. Built for the Raise Your Hack Prosus track.

## Tech Stack

- **Backend**: Rust with Axum framework
- **Frontend**: React.js
- **Database**: MongoDB (Atlas free tier) with Graph & Vector Search
- **AI/ML**: 
  - `openai/clip-vit-large-patch14` for multimodal embedding
  - `compound-beta` model (internally routes to LLAMA models)
  - Tavily integration for query decomposition & data analysis

## Infrastructure

- Azure Linux Standard_B2pls_v2 VM (4 GB/2 cores)
- Heroku Eco Dyno
- Cloudflare
- Third-party services: Twilio, SendGrid, Groq, Filebase

## Build Instructions

### API (Rust Backend)

1. Install Rust and Cargo:
   ```
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. Navigate to the API directory:
   ```
   cd api
   ```

3. Create a `.env` file with the following variables:
   ```
   MONGODB_URI=your_mongodb_connection_string
   GROQ_API_KEY=your_groq_api_key
   PORT=8080
   ```

4. Build and run the API:
   ```
   cargo build
   cargo run
   ```

### Web (React Frontend)

1. Install Node.js and npm (minimum version 16.x)

2. Navigate to the web directory:
   ```
   cd web
   ```

3. Install dependencies:
   ```
   npm install
   ```

4. Start the development server:
   ```
   npm start
   ```

5. Build for production:
   ```
   npm run build
   ```

### Clippy (Python Service)

1. Install Python 3.8+ and pip

2. Navigate to the clippy directory:
   ```
   cd clippy
   ```

3. Create a virtual environment and activate it:
   ```
   python -m venv venv
   source venv/bin/activate  # On Windows: venv\Scripts\activate
   ```

4. Install dependencies:
   ```
   pip install -r requirements.txt
   ```

5. Create a `.env` file with the required environment variables

6. Run the service:
   ```
   gunicorn -c gunicorn_config.py app:app
   ```