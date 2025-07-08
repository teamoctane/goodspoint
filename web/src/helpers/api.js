const API_BASE_URL = process.env.REACT_APP_API_URL || 'http://localhost:3000';

class APIError extends Error {
  constructor(message, status, response) {
    super(message);
    this.name = 'APIError';
    this.status = status;
    this.response = response;
  }
}

class APIClient {
  constructor() {
    this.baseURL = API_BASE_URL;
    this.csrfToken = null;
    this.retryCount = 3;
    this.retryDelay = 1000; // ms
  }

  /**
   * Make HTTP request to API
   * @private
   * @param {string} endpoint - API endpoint path
   * @param {Object} options - Fetch options
   * @param {number} attempt - Current retry attempt
   * @returns {Promise<any>} API response
   */
  async request(endpoint, options = {}, attempt = 1) {
    const url = `${this.baseURL}${endpoint}`;
    const config = {
      credentials: 'include',
      headers: {
        'Content-Type': 'application/json',
        ...options.headers,
      },
      ...options,
    };

    if (this.csrfToken && ['POST', 'PUT', 'DELETE'].includes(config.method?.toUpperCase())) {
      config.headers['x-authenticity-token'] = this.csrfToken;
    }

    try {
      const response = await fetch(url, config);
      
      if (response.status === 403 && attempt === 1) {
        await this.getCsrfToken();
        return this.request(endpoint, options, attempt + 1);
      }

      if (!response.ok) {
        const errorText = await response.text();
        let errorMessage = `HTTP ${response.status}: ${errorText}`;
        
        try {
          const errorJson = JSON.parse(errorText);
          errorMessage = errorJson.message || errorJson.error || errorMessage;
        } catch (e) {
        }
        
        throw new APIError(errorMessage, response.status, response);
      }

      const contentType = response.headers.get('content-type');
      if (contentType && contentType.includes('application/json')) {
        return await response.json();
      }
      
      return await response.text();
    } catch (error) {
      if (error instanceof APIError) {
        throw error;
      }

      if (attempt < this.retryCount && (
        error.name === 'TypeError' || 
        error.name === 'NetworkError' ||
        error.message.includes('fetch')
      )) {
        await new Promise(resolve => setTimeout(resolve, this.retryDelay));
        return this.request(endpoint, options, attempt + 1);
      }

      throw new APIError(`Network error: ${error.message}`, 0, null);
    }
  }

  /**
   * Get CSRF token from server
   * Call this before making any POST/PUT/DELETE requests
   * @returns {Promise<string>} CSRF token
   */
  async getCsrfToken() {
    try {
      const response = await this.request('/auth/csrf_token');
      this.csrfToken = response.csrf_token;
      return this.csrfToken;
    } catch (error) {
      throw error;
    }
  }

  /**
   * Check if user is authenticated
   * @returns {Promise<boolean>} Authentication status
   */
  async isAuthenticated() {
    try {
      await this.auth_getUser();
      return true;
    } catch (error) {
      return false;
    }
  }


  /**
   * Register a new user
   * @param {Object} userData - User registration data
   * @param {string} userData.email - User email
   * @param {string} userData.password - User password
   * @param {string} userData.username - Username
   * @returns {Promise<Object>} Registration response
   */
  async auth_register(userData) {
    return this.request('/auth/register', {
      method: 'POST',
      body: JSON.stringify(userData),
    });
  }

  /**
   * Login user
   * @param {Object} credentials - Login credentials
   * @param {string} credentials.email - User email
   * @param {string} credentials.password - User password
   * @returns {Promise<Object>} Login response
   */
  async auth_login(credentials) {
    return this.request('/auth/login', {
      method: 'POST',
      body: JSON.stringify(credentials),
    });
  }

  /**
   * Logout current user
   * @returns {Promise<Object>} Logout response
   */
  async auth_logout() {
    return this.request('/auth/logout', {
      method: 'POST',
    });
  }

  /**
   * Get current user information
   * @returns {Promise<Object>} User data
   */
  async auth_getUser() {
    return this.request('/auth/user');
  }


  /**
   * Create a new product
   * @param {Object} productData - Product data
   * @param {string} productData.title - Product title (ASCII only)
   * @param {string} productData.description - Product description (ASCII only)
   * @param {string} productData.product_type - 'new' or 'used'
   * @param {string} productData.category - Product category
   * @param {Array<string>} productData.tags - Product tags (ASCII only)
   * @param {Object} productData.quantity - Quantity limits
   * @param {number} productData.quantity.min_quantity - Minimum quantity
   * @param {number} productData.quantity.max_quantity - Maximum quantity
   * @param {string} [productData.price] - Product price
   * @param {File} [thumbnailFile] - Thumbnail image file
   * @param {Array<File>} [galleryFiles] - Gallery image files
   * @returns {Promise<Object>} Created product
   */
  async products_create(productData, thumbnailFile = null, galleryFiles = []) {
    const formData = new FormData();
    formData.append('product', JSON.stringify(productData));
    
    if (thumbnailFile) {
      formData.append('thumbnail', thumbnailFile);
    }
    
    galleryFiles.forEach((file, index) => {
      formData.append(`gallery_${index}`, file);
    });

    return this.request('/seller/products/create', {
      method: 'POST',
      headers: {}, // Remove Content-Type to let browser set it with boundary
      body: formData,
    });
  }

  /**
   * List user's products
   * @param {Object} [params] - Query parameters
   * @param {number} [params.limit] - Number of products to return
   * @param {number} [params.offset] - Number of products to skip
   * @returns {Promise<Object>} Products list response
   */
  async products_list(params = {}) {
    const searchParams = new URLSearchParams(params);
    const queryString = searchParams.toString();
    const endpoint = queryString ? `/seller/products/list?${queryString}` : '/seller/products/list';
    return this.request(endpoint);
  }

  /**
   * Get user's product by ID
   * @param {string} productId - Product ID
   * @returns {Promise<Object>} Product data
   */
  async products_get(productId) {
    return this.request(`/seller/products/${productId}`);
  }

  /**
   * Get public product by ID (no authentication required)
   * @param {string} productId - Product ID
   * @returns {Promise<Object>} Product data
   */
  async products_getPublic(productId) {
    return this.request(`/products/${productId}`);
  }

  /**
   * Update product
   * @param {string} productId - Product ID
   * @param {Object} updateData - Fields to update
   * @param {string} [updateData.title] - New title (ASCII only)
   * @param {string} [updateData.description] - New description (ASCII only)
   * @param {string} [updateData.product_type] - New product type
   * @param {string} [updateData.category] - New category
   * @param {Array<string>} [updateData.tags] - New tags (ASCII only)
   * @param {Object} [updateData.quantity] - New quantity limits
   * @param {string} [updateData.price] - New price
   * @returns {Promise<Object>} Updated product
   */
  async products_update(productId, updateData) {
    return this.request(`/seller/products/${productId}`, {
      method: 'PUT',
      body: JSON.stringify(updateData),
    });
  }

  /**
   * Delete product
   * @param {string} productId - Product ID
   * @returns {Promise<Object>} Deletion response
   */
  async products_delete(productId) {
    return this.request(`/seller/products/${productId}`, {
      method: 'DELETE',
    });
  }


  /**
   * Get product gallery
   * @param {string} productId - Product ID
   * @returns {Promise<Object>} Gallery data
   */
  async gallery_get(productId) {
    return this.request(`/seller/products/${productId}/gallery`);
  }

  /**
   * Replace entire gallery
   * @param {string} productId - Product ID
   * @param {Array<File>} files - New gallery files (max 6)
   * @returns {Promise<Object>} Gallery response
   */
  async gallery_replace(productId, files) {
    const formData = new FormData();
    files.forEach((file, index) => {
      formData.append(`file_${index}`, file);
    });

    return this.request(`/seller/products/${productId}/gallery/replace`, {
      method: 'POST',
      headers: {},
      body: formData,
    });
  }

  /**
   * Add items to gallery
   * @param {string} productId - Product ID
   * @param {Array<File>} files - Files to add
   * @returns {Promise<Object>} Gallery response
   */
  async gallery_add(productId, files) {
    const formData = new FormData();
    files.forEach((file, index) => {
      formData.append(`file_${index}`, file);
    });

    return this.request(`/seller/products/${productId}/gallery/add`, {
      method: 'POST',
      headers: {},
      body: formData,
    });
  }

  /**
   * Reorder gallery items
   * @param {string} productId - Product ID
   * @param {Array<string>} itemIds - Array of item IDs in new order
   * @returns {Promise<Object>} Reorder response
   */
  async gallery_reorder(productId, itemIds) {
    return this.request(`/seller/products/${productId}/gallery/reorder`, {
      method: 'POST',
      body: JSON.stringify({ item_ids: itemIds }),
    });
  }


  /**
   * Get product questions
   * @param {string} productId - Product ID
   * @returns {Promise<Object>} Questions data
   */
  async questions_get(productId) {
    return this.request(`/seller/products/${productId}/questions`);
  }

  /**
   * Set product questions
   * @param {string} productId - Product ID
   * @param {Object} questions - Questions data
   * @param {Array<Object>} questions.questions - Array of question objects
   * @param {string} questions.questions[].id - Question ID
   * @param {string} questions.questions[].question - Question text (ASCII only)
   * @param {string} questions.questions[].question_type - 'yes_no' or 'free_response'
   * @param {boolean} questions.questions[].mandatory - Whether question is mandatory
   * @returns {Promise<Object>} Questions response
   */
  async questions_set(productId, questions) {
    return this.request(`/seller/products/${productId}/questions/set`, {
      method: 'POST',
      body: JSON.stringify(questions),
    });
  }

  /**
   * Generate questions using AI
   * @param {string} productId - Product ID
   * @param {string} description - Product description for AI to use (ASCII only)
   * @returns {Promise<Object>} Generated questions
   */
  async questions_generate(productId, description) {
    return this.request(`/seller/products/${productId}/questions/generate`, {
      method: 'POST',
      body: JSON.stringify({ description }),
    });
  }


  /**
   * Get API root information
   * @returns {Promise<Object>} Root response
   */
  async root() {
    return this.request('/');
  }


  /**
   * Search for products using text and/or images
   * @param {Object} searchParams - Search parameters
   * @param {string} [searchParams.query] - Search query text
   * @param {string} [searchParams.category] - Product category filter
   * @param {string} [searchParams.product_type] - Product type filter ('new' or 'used')
   * @param {number} [searchParams.price_min] - Minimum price filter
   * @param {number} [searchParams.price_max] - Maximum price filter
   * @param {string} [searchParams.mode] - Search mode ('vector', 'text', or 'combined')
   * @param {string} [searchParams.sort] - Sort by ('relevance', 'price', 'created_at', 'popularity')
   * @param {string} [searchParams.sort_order] - Sort order ('asc' or 'desc')
   * @param {number} [searchParams.limit] - Number of results to return
   * @param {number} [searchParams.offset] - Number of results to skip
   * @param {boolean} [searchParams.use_ai_enhancement] - Whether to use AI to enhance the query
   * @param {string} [searchParams.conversation_id] - Conversation ID for context
   * @param {boolean} [searchParams.should_refine] - Whether to refine the query before searching
   * @param {Array<File>} [imageFiles] - Image files for multimodal search (max 2 images, 5MB each)
   * @returns {Promise<Object>} Search results
   */
  async search_products(searchParams = {}, imageFiles = []) {
    if (imageFiles.length > 2) {
      throw new Error('Cannot search with more than 2 images');
    }

    for (const file of imageFiles) {
      if (file.size > 5 * 1024 * 1024) { // 5MB limit
        throw new Error('Image file size cannot exceed 5MB');
      }
    }

    if (imageFiles.length > 0) {
      const formData = new FormData();
      
      Object.keys(searchParams).forEach(key => {
        if (searchParams[key] !== undefined && searchParams[key] !== null) {
          formData.append(key, searchParams[key].toString());
        }
      });

      imageFiles.forEach((file, index) => {
        formData.append(`image_${index}`, file);
      });

      return this.request('/search', {
        method: 'POST',
        headers: {}, // Remove Content-Type to let browser set it with boundary
        body: formData,
      });
    } else {
      const searchParams_url = new URLSearchParams();
      Object.keys(searchParams).forEach(key => {
        if (searchParams[key] !== undefined && searchParams[key] !== null) {
          searchParams_url.append(key, searchParams[key].toString());
        }
      });
      
      const queryString = searchParams_url.toString();
      const endpoint = queryString ? `/search?${queryString}` : '/search';
      return this.request(endpoint);
    }
  }

  /**
   * Simple text search (convenience method)
   * @param {string} query - Search query
   * @param {Object} [options] - Additional search options
   * @returns {Promise<Object>} Search results
   */
  async search_text(query, options = {}) {
    return this.search_products({
      query,
      mode: 'text',
      ...options,
    });
  }

  /**
   * Multimodal search with images (convenience method)
   * @param {string} [query] - Optional search query
   * @param {Array<File>} imageFiles - Image files for search
   * @param {Object} [options] - Additional search options
   * @returns {Promise<Object>} Search results
   */
  async search_multimodal(query, imageFiles, options = {}) {
    return this.search_products({
      query,
      mode: 'vector',
      ...options,
    }, imageFiles);
  }

  /**
   * Browse products with filters (no search query)
   * @param {Object} [filters] - Product filters
   * @returns {Promise<Object>} Browse results
   */
  async browse_products(filters = {}) {
    return this.search_products({
      mode: 'combined',
      ...filters,
    });
  }

  /**
   * Refine search query with AI assistance
   * @param {Object} refinementParams - Refinement parameters
   * @param {string} refinementParams.conversation_id - Conversation ID
   * @param {string} refinementParams.user_input - User's refinement input
   * @param {string} [refinementParams.previous_query] - Previous search query
   * @param {number} [refinementParams.search_results_count] - Number of results from previous search
   * @returns {Promise<Object>} Refinement response with suggestions
   */
  async search_refine(refinementParams) {
    return this.request('/search/refine', {
      method: 'POST',
      body: JSON.stringify(refinementParams),
    });
  }

  /**
   * Start a search conversation (convenience method)
   * @param {string} initialQuery - Initial search query
   * @returns {Promise<Object>} Search results with conversation context
   */
  async search_conversation_start(initialQuery) {
    return this.search_products({
      query: initialQuery,
      should_refine: true,
      use_ai_enhancement: true,
    });
  }

  /**
   * Continue a search conversation
   * @param {string} conversationId - Conversation ID
   * @param {string} userInput - User's follow-up input
   * @param {string} [previousQuery] - Previous search query
   * @param {number} [resultsCount] - Number of results from previous search
   * @returns {Promise<Object>} Refinement suggestions or search results
   */
  async search_conversation_continue(conversationId, userInput, previousQuery, resultsCount) {
    const refinement = await this.search_refine({
      conversation_id: conversationId,
      user_input: userInput,
      previous_query: previousQuery,
      search_results_count: resultsCount,
    });

    if (refinement.should_search_immediately && refinement.refined_query) {
      return this.search_products({
        query: refinement.refined_query,
        conversation_id: conversationId,
        use_ai_enhancement: false, // Already refined
      });
    }

    return refinement;
  }
}

const api = new APIClient();

export default api;


/**
 * Product Types
 * @enum {string}
 */
export const ProductType = {
  NEW: 'new',
  USED: 'used',
};

/**
 * Product Categories
 * @enum {string}
 */
export const ProductCategory = {
  SMARTPHONES: 'Smartphones',
  COMPUTERS: 'Computers',
  AUDIO: 'Audio',
  CAMERAS: 'Cameras',
  GAMING: 'Gaming',
  WEARABLES: 'Wearables',
  HOME_ELECTRONICS: 'HomeElectronics',
  MENS_CLOTHING: 'MensClothing',
  WOMENS_CLOTHING: 'WomensClothing',
  SHOES: 'Shoes',
  ACCESSORIES: 'Accessories',
  JEWELRY: 'Jewelry',
  BAGS: 'Bags',
  BEAUTY: 'Beauty',
  FURNITURE: 'Furniture',
  HOME_DECOR: 'HomeDecor',
  KITCHEN: 'Kitchen',
  GARDEN: 'Garden',
  HOME_TOOLS: 'HomeTools',
  HOME_IMPROVEMENT: 'HomeImprovement',
  FITNESS_EQUIPMENT: 'FitnessEquipment',
  OUTDOOR_GEAR: 'OutdoorGear',
  SPORTS_EQUIPMENT: 'SportsEquipment',
  BICYCLES: 'Bicycles',
  WATER_SPORTS: 'WaterSports',
  WINTER_SPORTS: 'WinterSports',
  CAR_PARTS: 'CarParts',
  MOTORCYCLES: 'Motorcycles',
  AUTO_TOOLS: 'AutoTools',
  CAR_ACCESSORIES: 'CarAccessories',
  BOOKS: 'Books',
  MUSIC: 'Music',
  MOVIES: 'Movies',
  VIDEO_GAMES: 'VideoGames',
  HEALTH_EQUIPMENT: 'HealthEquipment',
  PERSONAL_CARE: 'PersonalCare',
  SUPPLEMENTS: 'Supplements',
  MEDICAL_DEVICES: 'MedicalDevices',
  BABY_CLOTHING: 'BabyClothing',
  TOYS: 'Toys',
  BABY_GEAR: 'BabyGear',
  KIDS_ELECTRONICS: 'KidsElectronics',
  COLLECTIBLES: 'Collectibles',
  ANTIQUES: 'Antiques',
  ART: 'Art',
  CRAFTS: 'Crafts',
  OFFICE_SUPPLIES: 'OfficeSupplies',
  INDUSTRIAL_EQUIPMENT: 'IndustrialEquipment',
  BUSINESS_EQUIPMENT: 'BusinessEquipment',
  OTHER: 'Other',
};

/**
 * Question Types
 * @enum {string}
 */
export const QuestionType = {
  YES_NO: 'yes_no',
  FREE_RESPONSE: 'free_response',
};

/**
 * Search Modes
 * @enum {string}
 */
export const SearchMode = {
  VECTOR: 'vector',
  TEXT: 'text',
  COMBINED: 'combined',
};

/**
 * Search Sort Options
 * @enum {string}
 */
export const SearchSort = {
  RELEVANCE: 'relevance',
  PRICE: 'price',
  CREATED_AT: 'created_at',
  POPULARITY: 'popularity',
};

/**
 * Sort Orders
 * @enum {string}
 */
export const SortOrder = {
  ASC: 'asc',
  DESC: 'desc',
};
