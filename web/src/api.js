/**
 * GoodsPoint API Client
 * 
 * A comprehensive JavaScript client for the GoodsPoint API using Fetch API.
 * Covers all endpoints including Authentication, Products, Chat, Orders, Search, and Recommendations.
 * 
 * Author: Generated for GoodsPoint
 * Date: July 8, 2025
 */

class GoodsPointAPI {
    constructor(baseURL = '', authCookie = null) {
        this.baseURL = baseURL;
        this.authCookie = authCookie;
    }

    /**
     * Set authentication cookie for API requests (used by the backend)
     * @param {string} cookie - Authentication cookie value
     */
    setAuthCookie(cookie) {
        this.authCookie = cookie;
    }

    /**
     * Make authenticated API request
     * @param {string} endpoint - API endpoint
     * @param {Object} options - Fetch options
     * @returns {Promise<Object>} - API response
     */
    async request(endpoint, options = {}) {
        const url = `${this.baseURL}${endpoint}`;
        const headers = {
            'Content-Type': 'application/json',
            ...options.headers,
        };

        const config = {
            credentials: 'include',
            ...options,
            headers,
        };

        try {
            const response = await fetch(url, config);
            
            if (!response.ok) {
                const errorData = await response.json().catch(() => ({}));
                throw new Error(errorData.message || `HTTP ${response.status}: ${response.statusText}`);
            }

            const data = await response.json();
            
            // Update auth cookie if provided in response (for development)
            if (response.headers.get('set-cookie')) {
                const cookies = response.headers.get('set-cookie');
                const authMatch = cookies.match(/GOODSPOINT_AUTHENTICATION=([^;]+)/);
                if (authMatch) {
                    this.authCookie = authMatch[1];
                }
            }

            return data;
        } catch (error) {
            console.error('API Request failed:', error);
            throw error;
        }
    }

    /**
     * Make multipart form data request
     * @param {string} endpoint - API endpoint
     * @param {FormData} formData - Form data to send
     * @param {string} method - HTTP method (default: 'POST')
     * @returns {Promise<Object>} - API response
     */
    async multipartRequest(endpoint, formData, method = 'POST') {
        const url = `${this.baseURL}${endpoint}`;
        const headers = {};

        try {
            const response = await fetch(url, {
                method,
                credentials: 'include',
                headers,
                body: formData,
            });

            if (!response.ok) {
                const errorData = await response.json().catch(() => ({}));
                throw new Error(errorData.message || `HTTP ${response.status}: ${response.statusText}`);
            }

            return await response.json();
        } catch (error) {
            console.error('Multipart API Request failed:', error);
            throw error;
        }
    }

    // ============================
    // ROOT API ENDPOINT
    // ============================

    /**
     * Get API status
     * @returns {Promise<Object>} Status message
     * 
     * Response structure:
     * {
     *   message: "ok"
     * }
     */
    async getStatus() {
        return this.request('/');
    }

    // CSRF protection has been removed

    // ============================
    // AUTHENTICATION API ENDPOINTS
    // ============================

    /**
     * Register a new user
     * @param {Object} userData - User registration data
     * @param {string} userData.username - Username (required)
     * @param {string} userData.email - Email address (required)
     * @param {string} userData.password - Password (required)
     * @returns {Promise<Object>} Registration response
     * 
     * Response structure:
     * {
     *   status: "ok",
     *   message: "Account created successfully. Please check your email for verification code.",
     *   user: {
     *     username: string,
     *     email: string,
     *     uid: string
     *   }
     * }
     */
    async register(userData) {
        return this.request('/auth/register', {
            method: 'POST',
            body: JSON.stringify(userData),
        });
    }

    /**
     * Login user with username/email and password
     * @param {Object} credentials - Login credentials
     * @param {string} credentials.username - Username (optional if email provided)
     * @param {string} credentials.email - Email (optional if username provided)
     * @param {string} credentials.password - Password (required)
     * @returns {Promise<Object>} Login response
     * 
     * Response structure:
     * {
     *   status: "ok"
     * }
     * 
     * Note: Authentication cookie is set automatically on successful login
     */
    async login(credentials) {
        return this.request('/auth/login', {
            method: 'POST',
            body: JSON.stringify(credentials),
        });
    }

    /**
     * Logout current user
     * @returns {Promise<Object>} Logout response
     * 
     * Response structure:
     * {
     *   status: "ok"
     * }
     */
    async logout() {
        return this.request('/auth/logout', {
            method: 'POST',
        });
    }

    /**
     * Get current user information
     * @returns {Promise<Object>} User information
     * 
     * Response structure:
     * {
     *   user: {
     *     username: string,
     *     email: string,
     *     uid: string
     *   }
     * }
     */
    async getCurrentUser() {
        return this.request('/auth/user');
    }

    /**
     * Change user password
     * @param {Object} passwordData - Password change data
     * @param {string} passwordData.old_password - Current password
     * @param {string} passwordData.new_password - New password
     * @returns {Promise<Object>} Password change response
     * 
     * Response structure:
     * {
     *   success: boolean,
     *   message: string
     * }
     */
    async changePassword(passwordData) {
        return this.request('/auth/change-password', {
            method: 'POST',
            body: JSON.stringify(passwordData),
        });
    }

    /**
     * Send email OTP for verification
     * @param {string} email - Email address to send OTP to
     * @returns {Promise<Object>} OTP send response
     * 
     * Response structure:
     * {
     *   success: boolean,
     *   message: "OTP sent to email"
     * }
     */
    async sendEmailOTP(email) {
        return this.request('/auth/send-email-otp', {
            method: 'POST',
            body: JSON.stringify({ email }),
        });
    }

    /**
     * Verify email OTP
     * @param {Object} otpData - OTP verification data
     * @param {string} otpData.email - Email address
     * @param {string} otpData.otp - OTP code
     * @returns {Promise<Object>} Verification response
     * 
     * Response structure:
     * {
     *   success: boolean,
     *   message: "Email verified successfully"
     * }
     */
    async verifyEmailOTP(otpData) {
        return this.request('/auth/verify-email-otp', {
            method: 'POST',
            body: JSON.stringify(otpData),
        });
    }

    /**
     * Send WhatsApp OTP
     * @param {string} whatsappNumber - WhatsApp number
     * @returns {Promise<Object>} OTP send response
     * 
     * Response structure:
     * {
     *   success: boolean,
     *   message: "OTP sent to WhatsApp"
     * }
     */
    async sendWhatsAppOTP(whatsappNumber) {
        return this.request('/auth/send-whatsapp-otp', {
            method: 'POST',
            body: JSON.stringify({ whatsapp_number: whatsappNumber }),
        });
    }

    /**
     * Verify WhatsApp OTP
     * @param {Object} otpData - OTP verification data
     * @param {string} otpData.whatsapp_number - WhatsApp number
     * @param {string} otpData.otp - OTP code
     * @returns {Promise<Object>} Verification response
     * 
     * Response structure:
     * {
     *   success: boolean,
     *   message: "WhatsApp verified successfully"
     * }
     */
    async verifyWhatsAppOTP(otpData) {
        return this.request('/auth/verify-whatsapp-otp', {
            method: 'POST',
            body: JSON.stringify(otpData),
        });
    }
    
    /**
     * Get WhatsApp verification status for current user
     * @returns {Promise<Object>} WhatsApp status
     * 
     * Response structure:
     * {
     *   whatsapp_verified: boolean,
     *   whatsapp_number: string | null
     * }
     */
    async getWhatsAppStatus() {
        return this.request('/auth/whatsapp-status');
    }

    // ============================
    // PRODUCT API ENDPOINTS
    // ============================

    /**
     * Create a new product
     * @param {Object} productData - Product creation data
     * @param {string} productData.title - Product title (max 200 characters)
     * @param {string} productData.description - Product description (max 2000 characters)
     * @param {string} productData.product_type - "new" or "used"
     * @param {string} productData.purchase_type - "buy_now" or "inquire"
     * @param {string} productData.category - Product category (see ProductCategory enum)
     * @param {string[]} productData.tags - Product tags (max 32 tags, 50 chars each)
     * @param {Object} productData.quantity - Quantity constraints
     * @param {number} productData.quantity.min_quantity - Minimum quantity
     * @param {number} productData.quantity.max_quantity - Maximum quantity
     * @param {number} productData.price - Product price
     * @param {Object} productData.custom_questions - Optional custom questions
     * @returns {Promise<Object>} Created product
     * 
     * Product categories include: Smartphones, Computers, Audio, Cameras, Gaming, 
     * Wearables, HomeElectronics, MensClothing, WomensClothing, etc.
     */
    async createProduct(productData) {
        return this.request('/seller/products/create', {
            method: 'POST',
            body: JSON.stringify(productData),
        });
    }

    /**
     * Get list of current user's products
     * @param {Object} options - Query options
     * @param {number} options.limit - Number of products to fetch (default: 20, max: 100)
     * @param {number} options.offset - Number of products to skip (default: 0)
     * @returns {Promise<Array>} Products array
     * 
     * Response structure:
     * [
     *   {
     *     product_id: string,
     *     title: string,
     *     product_type: "new" | "used",
     *     quantity: { min_quantity: number, max_quantity: number },
     *     created_at: number,
     *     enabled: boolean,
     *     thumbnail_url: string | null
     *   }
     * ]
     */
    async listMyProducts(options = {}) {
        const params = new URLSearchParams();
        if (options.limit) params.append('limit', options.limit.toString());
        if (options.offset) params.append('offset', options.offset.toString());
        
        const query = params.toString() ? `?${params.toString()}` : '';
        return this.request(`/seller/products/list${query}`);
    }

    /**
     * Get product details (public endpoint)
     * @param {string} productId - ID of the product
     * @returns {Promise<Object>} Product details
     * 
     * Response structure:
     * {
     *   product_id: string,
     *   user_id: string,
     *   username: string,
     *   title: string,
     *   description: string,
     *   product_type: "new" | "used",
     *   purchase_type: "buy_now" | "inquire",
     *   category: string,
     *   tags: string[],
     *   quantity: { min_quantity: number, max_quantity: number },
     *   price: number,
     *   custom_questions: object | null,
     *   gallery: array,
     *   thumbnail_url: string | null,
     *   created_at: number,
     *   updated_at: number,
     *   enabled: boolean
     * }
     */
    async getProduct(productId) {
        return this.request(`/products/${productId}`);
    }

    /**
     * Get current user's product details
     * @param {string} productId - ID of the product
     * @returns {Promise<Object>} Product details
     */
    async getUserProduct(productId) {
        return this.request(`/seller/products/${productId}`);
    }

    /**
     * Update product details
     * @param {string} productId - ID of the product
     * @param {Object} updateData - Fields to update (same structure as createProduct, all optional)
     * @returns {Promise<Object>} Updated product
     */
    async updateProduct(productId, updateData) {
        return this.request(`/seller/products/${productId}`, {
            method: 'PUT',
            body: JSON.stringify(updateData),
        });
    }

    /**
     * Delete a product
     * @param {string} productId - ID of the product
     * @returns {Promise<Object>} Deletion response
     */
    async deleteProduct(productId) {
        return this.request(`/seller/products/${productId}`, {
            method: 'DELETE',
        });
    }

    /**
     * Buy a product immediately (create order with buy-now functionality)
     * @param {string} productId - ID of the product to buy
     * @param {number} quantity - Quantity to purchase
     * @returns {Promise<Object>} Order object
     * 
     * Response structure:
     * {
     *   order_id: string,
     *   product_id: string,
     *   seller_id: string,
     *   buyer_id: string,
     *   quantity: number,
     *   price: number,
     *   status: "unpaid" | "delivery_pending",
     *   created_at: number,
     *   updated_at: number
     * }
     */
    async buyNowProduct(productId, quantity) {
        return this.request('/products/buy-now', {
            method: 'POST',
            body: JSON.stringify({ 
                product_id: productId, 
                quantity: quantity 
            }),
        });
    }

    // ============================
    // PRODUCT GALLERY API ENDPOINTS
    // ============================

    /**
     * Get gallery items for a product
     * @param {string} productId - ID of the product
     * @returns {Promise<Object>} Gallery items
     * 
     * Response structure:
     * {
     *   status: "ok",
     *   gallery: [
     *     {
     *       id: string,
     *       item_type: "picture" | "video" | "obj" | "other",
     *       url: string,
     *       size: number,
     *       order: number,
     *       upload_timestamp: number
     *     }
     *   ]
     * }
     */
    async getProductGallery(productId) {
        return this.request(`/seller/products/${productId}/gallery`);
    }

    /**
     * Replace all gallery items for a product
     * @param {string} productId - ID of the product
     * @param {File[]} files - Array of files to upload (max 6 items, 50MB each)
     * @returns {Promise<Object>} Updated gallery
     * 
     * Allowed file types: image/*, video/*, model/*, application/octet-stream
     */
    async replaceProductGallery(productId, files) {
        const formData = new FormData();
        files.forEach(file => {
            formData.append('gallery', file);
        });
        
        return this.multipartRequest(`/seller/products/${productId}/gallery/replace`, formData);
    }

    /**
     * Add gallery items to a product
     * @param {string} productId - ID of the product
     * @param {File[]} files - Array of files to upload
     * @returns {Promise<Object>} Updated gallery
     */
    async addProductGalleryItems(productId, files) {
        const formData = new FormData();
        files.forEach(file => {
            formData.append('gallery', file);
        });
        
        return this.multipartRequest(`/seller/products/${productId}/gallery/add`, formData);
    }

    /**
     * Reorder gallery items for a product
     * @param {string} productId - ID of the product
     * @param {string[]} itemIds - Array of gallery item IDs in desired order
     * @returns {Promise<Object>} Reordered gallery
     */
    async reorderProductGallery(productId, itemIds) {
        return this.request(`/seller/products/${productId}/gallery/reorder`, {
            method: 'POST',
            body: JSON.stringify({ item_ids: itemIds }),
        });
    }

    // ============================
    // PRODUCT QUESTIONS API ENDPOINTS
    // ============================

    /**
     * Get custom questions for a product
     * @param {string} productId - ID of the product
     * @returns {Promise<Object>} Product questions
     * 
     * Response structure:
     * {
     *   status: "ok",
     *   questions: {
     *     questions: [
     *       {
     *         id: string,
     *         question: string,
     *         question_type: "yes_no" | "free_response",
     *         mandatory: boolean
     *       }
     *     ]
     *   }
     * }
     */
    async getProductQuestions(productId) {
        return this.request(`/seller/products/${productId}/questions`);
    }

    /**
     * Set custom questions for a product
     * @param {string} productId - ID of the product
     * @param {Object} questions - Questions object
     * @param {Array} questions.questions - Array of question objects
     * @returns {Promise<Object>} Updated questions
     * 
     * Question object structure:
     * {
     *   id: string,
     *   question: string, // max 1300 characters
     *   question_type: "yes_no" | "free_response",
     *   mandatory: boolean
     * }
     */
    async setProductQuestions(productId, questions) {
        return this.request(`/seller/products/${productId}/questions/set`, {
            method: 'POST',
            body: JSON.stringify(questions),
        });
    }

    /**
     * Generate questions for a product using AI
     * @param {string} productId - ID of the product
     * @param {string} description - Description of what questions to generate
     * @returns {Promise<Object>} Generated questions
     * 
     * Response structure: Same as getProductQuestions
     * 
     * Example description: "I need to know the color and size preferences"
     */
    async generateProductQuestions(productId, description) {
        return this.request(`/seller/products/${productId}/questions/generate`, {
            method: 'POST',
            body: JSON.stringify({ description }),
        });
    }

    // ============================
    // CHAT API ENDPOINTS
    // ============================

    /**
     * Send a text message to another user
     * @param {string} otherUserId - ID of the recipient user
     * @param {string} content - Message content (max 4000 characters)
     * @returns {Promise<Object>} Message object
     * 
     * Response structure:
     * {
     *   status: "ok",
     *   message: {
     *     message_id: string,
     *     sender_id: string,
     *     message_type: "text" | "attachment" | "query" | "quote",
     *     content: string,
     *     attachment: null,
     *     created_at: number,
     *     updated_at: number,
     *     is_edited: boolean
     *   }
     * }
     */
    async sendTextMessage(otherUserId, content) {
        const formData = new FormData();
        formData.append('content', content);
        
        return this.multipartRequest(`/chat/${otherUserId}/messages`, formData);
    }

    /**
     * Send an attachment message to another user
     * @param {string} otherUserId - ID of the recipient user
     * @param {File} file - File to attach (max 50MB)
     * @returns {Promise<Object>} Message object with attachment
     * 
     * Allowed file types: image/jpeg, image/jpg, image/png, image/gif, image/webp,
     * video/mp4, video/quicktime, video/x-msvideo, text/plain, application/octet-stream
     */
    async sendAttachmentMessage(otherUserId, file) {
        const formData = new FormData();
        formData.append('attachment', file);
        
        return this.multipartRequest(`/chat/${otherUserId}/messages`, formData);
    }

    /**
     * Get messages from a conversation with another user
     * @param {string} otherUserId - ID of the other user
     * @param {Object} options - Query options
     * @param {number} options.limit - Number of messages to fetch (default: 64, max: 100)
     * @param {string} options.before - Message ID to fetch messages before
     * @returns {Promise<Object>} Messages array
     * 
     * Response structure:
     * {
     *   status: "ok",
     *   messages: [
     *     {
     *       message_id: string,
     *       sender_id: string,
     *       message_type: "text" | "attachment" | "query" | "quote",
     *       content: string | null,
     *       attachment: {
     *         id: string,
     *         file_name: string,
     *         content_type: string,
     *         url: string,
     *         size: number,
     *         upload_timestamp: number
     *       } | null,
     *       created_at: number,
     *       updated_at: number,
     *       is_edited: boolean
     *     }
     *   ]
     * }
     */
    async getMessages(otherUserId, options = {}) {
        const params = new URLSearchParams();
        if (options.limit) params.append('limit', options.limit.toString());
        if (options.before) params.append('before', options.before);
        
        const query = params.toString() ? `?${params.toString()}` : '';
        return this.request(`/chat/${otherUserId}/messages${query}`);
    }

    /**
     * Edit a text message
     * @param {string} messageId - ID of the message to edit
     * @param {string} newContent - New message content
     * @returns {Promise<Object>} Updated message object
     */
    async editMessage(messageId, newContent) {
        return this.request(`/chat/messages/${messageId}/edit`, {
            method: 'PUT',
            body: JSON.stringify({ content: newContent }),
        });
    }

    /**
     * Get all conversations for the current user
     * @returns {Promise<Object>} Conversations array
     * 
     * Response structure:
     * {
     *   status: "ok",
     *   conversations: [
     *     {
     *       conversation_id: string,
     *       other_participant_id: string,
     *       created_at: number,
     *       last_message_at: number
     *     }
     *   ]
     * }
     */
    async getConversations() {
        return this.request('/chat/conversations');
    }

    /**
     * Get edit history for a message
     * @param {string} messageId - ID of the message
     * @returns {Promise<Object>} Edit history array
     * 
     * Response structure:
     * {
     *   status: "ok",
     *   edit_history: [
     *     {
     *       content: string | null,
     *       attachment: object | null,
     *       edited_at: number,
     *       username: string | null
     *     }
     *   ]
     * }
     */
    async getMessageEditHistory(messageId) {
        return this.request(`/chat/messages/${messageId}/history`);
    }

    /**
     * Create an order from a quote message
     * @param {string} messageId - ID of the quote message
     * @returns {Promise<Object>} Order object
     */
    async createOrderFromQuote(messageId) {
        return this.request('/chat/quotes/create-order', {
            method: 'POST',
            body: JSON.stringify({ message_id: messageId }),
        });
    }

    // ============================
    // ORDERS API ENDPOINTS
    // ============================

    /**
     * List orders for the current user (as buyer)
     * @param {Object} options - Query options
     * @param {number} options.limit - Number of orders to fetch (default: 20, max: 100)
     * @param {number} options.offset - Number of orders to skip (default: 0)
     * @returns {Promise<Array>} Orders array
     * 
     * Response structure:
     * [
     *   {
     *     order_id: string,
     *     product_id: string,
     *     seller_id: string,
     *     buyer_id: string,
     *     quantity: number,
     *     price: number,
     *     status: "unpaid" | "delivery_pending",
     *     created_at: number,
     *     updated_at: number
     *   }
     * ]
     */
    async listOrders(options = {}) {
        const params = new URLSearchParams();
        if (options.limit) params.append('limit', options.limit.toString());
        if (options.offset) params.append('offset', options.offset.toString());
        
        const query = params.toString() ? `?${params.toString()}` : '';
        return this.request(`/orders/list${query}`);
    }

    /**
     * List orders for the current user (as seller)
     * @param {Object} options - Query options
     * @param {number} options.limit - Number of orders to fetch (default: 20, max: 100)
     * @param {number} options.offset - Number of orders to skip (default: 0)
     * @returns {Promise<Array>} Orders array
     */
    async listSellerOrders(options = {}) {
        const params = new URLSearchParams();
        if (options.limit) params.append('limit', options.limit.toString());
        if (options.offset) params.append('offset', options.offset.toString());
        
        const query = params.toString() ? `?${params.toString()}` : '';
        return this.request(`/sellers/orders/list${query}`);
    }

    /**
     * Confirm an order (change status from unpaid to delivery_pending)
     * @param {string} orderId - ID of the order to confirm
     * @returns {Promise<Object>} Updated order object
     */
    async confirmOrder(orderId) {
        return this.request('/orders/confirm', {
            method: 'POST',
            body: JSON.stringify({ order_id: orderId }),
        });
    }

    // ============================
    // SEARCH API ENDPOINTS
    // ============================

    /**
     * Search products with text and/or images
     * @param {Object} searchData - Search parameters
     * @param {string} searchData.query - Search query text (optional)
     * @param {number} searchData.limit - Number of results to return (optional)
     * @param {boolean} searchData.force_original - Force original query without AI enhancement
     * @param {File[]} images - Array of image files to search with (max 2 images, 5MB each)
     * @returns {Promise<Object>} Search results
     * 
     * Response structure:
     * {
     *   results: [
     *     {
     *       product_id: string,
     *       title: string,
     *       description: string,
     *       product_type: "new" | "used",
     *       category: string,
     *       tags: string[],
     *       quantity: { min_quantity: number, max_quantity: number },
     *       price: string | null,
     *       thumbnail_url: string | null,
     *       created_at: number,
     *       similarity_score: number | null,
     *       username: string
     *     }
     *   ],
     *   total_count: number,
     *   enhanced_query: string | null,
     *   ai_enhancement_triggered: boolean,
     *   processing_time_ms: number,
     *   inferred_category: string | null
     * }
     */
    async searchProducts(searchData, images = []) {
        const formData = new FormData();
        
        if (searchData.query || searchData.limit !== undefined || searchData.force_original !== undefined) {
            formData.append('body', JSON.stringify(searchData));
        }
        
        images.forEach(image => {
            formData.append('images', image);
        });
        
        return this.multipartRequest('/products/search', formData);
    }



    // ============================
    // RECOMMENDATIONS API ENDPOINTS
    // ============================

    /**
     * Get personalized recommendations for the current user
     * @returns {Promise<Object>} Recommendations
     * 
     * Response structure:
     * {
     *   user_id: string,
     *   rows: [
     *     {
     *       title: string,
     *       products: [
     *         {
     *           product_id: string,
     *           title: string,
     *           price_in_inr: number | null,
     *           thumbnail_url: string | null,
     *           category: string,
     *           relevance_score: number
     *         }
     *       ]
     *     }
     *   ],
     *   generated_at: string
     * }
     */
    async getRecommendations() {
        return this.request('/homepage/recommendations');
    }

    // ============================
    // UTILITY METHODS
    // ============================

    /**
     * Upload multiple files with progress tracking
     * @param {string} endpoint - API endpoint
     * @param {File[]} files - Files to upload
     * @param {Function} onProgress - Progress callback (progress: number 0-100)
     * @returns {Promise<Object>} Upload result
     */
    async uploadWithProgress(endpoint, files, onProgress = null) {
        const formData = new FormData();
        files.forEach(file => {
            formData.append('gallery', file);
        });

        return new Promise((resolve, reject) => {
            const xhr = new XMLHttpRequest();
            
            if (onProgress) {
                xhr.upload.addEventListener('progress', (e) => {
                    if (e.lengthComputable) {
                        const progress = (e.loaded / e.total) * 100;
                        onProgress(progress);
                    }
                });
            }

            xhr.addEventListener('load', async () => {
                if (xhr.status >= 200 && xhr.status < 300) {
                    try {
                        const response = JSON.parse(xhr.responseText);
                        resolve(response);
                    } catch {
                        reject(new Error('Failed to parse response'));
                    }
                } else {
                    reject(new Error(`HTTP ${xhr.status}: ${xhr.statusText}`));
                }
            });

            xhr.addEventListener('error', () => {
                reject(new Error('Upload failed'));
            });

            xhr.open('POST', `${this.baseURL}${endpoint}`);
            
            if (this.authCookie) {
                xhr.setRequestHeader('Cookie', `GOODSPOINT_AUTHENTICATION=${this.authCookie}`);
            }

            xhr.send(formData);
        });
    }

    /**
     * Validate file for upload
     * @param {File} file - File to validate
     * @param {Object} options - Validation options
     * @param {number} options.maxSize - Maximum file size in bytes (default: 50MB)
     * @param {string[]} options.allowedTypes - Allowed MIME types
     * @returns {Object} Validation result { valid: boolean, error?: string }
     */
    validateFile(file, options = {}) {
        const maxSize = options.maxSize || 50 * 1024 * 1024; // 50MB
        const allowedTypes = options.allowedTypes || [
            'image/jpeg', 'image/jpg', 'image/png', 'image/gif', 'image/webp',
            'video/mp4', 'video/quicktime', 'video/x-msvideo',
            'text/plain', 'application/octet-stream',
            'model/obj', 'model/gltf+json', 'model/gltf-binary'
        ];

        if (file.size > maxSize) {
            return {
                valid: false,
                error: `File size (${(file.size / 1024 / 1024).toFixed(2)}MB) exceeds maximum allowed size (${(maxSize / 1024 / 1024).toFixed(2)}MB)`
            };
        }

        if (!allowedTypes.includes(file.type)) {
            return {
                valid: false,
                error: `File type "${file.type}" is not allowed`
            };
        }

        return { valid: true };
    }

    /**
     * Validate search images specifically
     * @param {File} file - Image file to validate
     * @returns {Object} Validation result { valid: boolean, error?: string }
     */
    validateSearchImage(file) {
        const maxSize = 5 * 1024 * 1024; // 5MB for search images
        const allowedTypes = [
            'image/jpeg', 'image/jpg', 'image/png', 'image/gif', 'image/webp'
        ];

        return this.validateFile(file, { maxSize, allowedTypes });
    }



    /**
     * Format file size for display
     * @param {number} bytes - File size in bytes
     * @returns {string} Formatted file size (e.g., "1.5 MB")
     */
    formatFileSize(bytes) {
        if (bytes === 0) return '0 Bytes';
        
        const k = 1024;
        const sizes = ['Bytes', 'KB', 'MB', 'GB'];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        
        return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
    }

    /**
     * Get product category display name
     * @param {string} category - Category enum value
     * @returns {string} Display name
     */
    getCategoryDisplayName(category) {
        const categoryMap = {
            'Smartphones': 'Smartphones',
            'Computers': 'Computers & Laptops',
            'Audio': 'Audio Equipment',
            'Cameras': 'Cameras & Photography',
            'Gaming': 'Gaming',
            'Wearables': 'Wearable Technology',
            'HomeElectronics': 'Home Electronics',
            'MensClothing': "Men's Clothing",
            'WomensClothing': "Women's Clothing",
            'UnisexClothing': 'Unisex Clothing',
            'Shoes': 'Shoes & Footwear',
            'Accessories': 'Accessories',
            'Jewelry': 'Jewelry',
            'Bags': 'Bags & Luggage',
            'Beauty': 'Beauty & Personal Care',
            'Furniture': 'Furniture',
            'HomeDecor': 'Home Decor',
            'Kitchen': 'Kitchen & Dining',
            'Garden': 'Garden & Outdoor',
            'HomeTools': 'Home Tools',
            'HomeImprovement': 'Home Improvement',
            'FitnessEquipment': 'Fitness Equipment',
            'OutdoorGear': 'Outdoor Gear',
            'SportsEquipment': 'Sports Equipment',
            'Bicycles': 'Bicycles',
            'WaterSports': 'Water Sports',
            'WinterSports': 'Winter Sports',
            'CarParts': 'Car Parts',
            'Motorcycles': 'Motorcycles',
            'AutoTools': 'Auto Tools',
            'CarAccessories': 'Car Accessories',
            'Books': 'Books',
            'Music': 'Music',
            'Movies': 'Movies & TV',
            'VideoGames': 'Video Games',
            'HealthEquipment': 'Health Equipment',
            'PersonalCare': 'Personal Care',
            'Supplements': 'Supplements',
            'MedicalDevices': 'Medical Devices',
            'BabyClothing': 'Baby Clothing',
            'Toys': 'Toys',
            'BabyGear': 'Baby Gear',
            'KidsElectronics': 'Kids Electronics',
            'Collectibles': 'Collectibles',
            'Antiques': 'Antiques',
            'Art': 'Art',
            'Crafts': 'Crafts',
            'OfficeSupplies': 'Office Supplies',
            'IndustrialEquipment': 'Industrial Equipment',
            'BusinessEquipment': 'Business Equipment',
            'Other': 'Other'
        };

        return categoryMap[category] || category;
    }

    /**
     * Get all available product categories
     * @returns {Array} Array of category objects with value and label
     */
    getProductCategories() {
        return [
            { value: 'Smartphones', label: 'Smartphones' },
            { value: 'Computers', label: 'Computers & Laptops' },
            { value: 'Audio', label: 'Audio Equipment' },
            { value: 'Cameras', label: 'Cameras & Photography' },
            { value: 'Gaming', label: 'Gaming' },
            { value: 'Wearables', label: 'Wearable Technology' },
            { value: 'HomeElectronics', label: 'Home Electronics' },
            { value: 'MensClothing', label: "Men's Clothing" },
            { value: 'WomensClothing', label: "Women's Clothing" },
            { value: 'UnisexClothing', label: 'Unisex Clothing' },
            { value: 'Shoes', label: 'Shoes & Footwear' },
            { value: 'Accessories', label: 'Accessories' },
            { value: 'Jewelry', label: 'Jewelry' },
            { value: 'Bags', label: 'Bags & Luggage' },
            { value: 'Beauty', label: 'Beauty & Personal Care' },
            { value: 'Furniture', label: 'Furniture' },
            { value: 'HomeDecor', label: 'Home Decor' },
            { value: 'Kitchen', label: 'Kitchen & Dining' },
            { value: 'Garden', label: 'Garden & Outdoor' },
            { value: 'HomeTools', label: 'Home Tools' },
            { value: 'HomeImprovement', label: 'Home Improvement' },
            { value: 'FitnessEquipment', label: 'Fitness Equipment' },
            { value: 'OutdoorGear', label: 'Outdoor Gear' },
            { value: 'SportsEquipment', label: 'Sports Equipment' },
            { value: 'Bicycles', label: 'Bicycles' },
            { value: 'WaterSports', label: 'Water Sports' },
            { value: 'WinterSports', label: 'Winter Sports' },
            { value: 'CarParts', label: 'Car Parts' },
            { value: 'Motorcycles', label: 'Motorcycles' },
            { value: 'AutoTools', label: 'Auto Tools' },
            { value: 'CarAccessories', label: 'Car Accessories' },
            { value: 'Books', label: 'Books' },
            { value: 'Music', label: 'Music' },
            { value: 'Movies', label: 'Movies & TV' },
            { value: 'VideoGames', label: 'Video Games' },
            { value: 'HealthEquipment', label: 'Health Equipment' },
            { value: 'PersonalCare', label: 'Personal Care' },
            { value: 'Supplements', label: 'Supplements' },
            { value: 'MedicalDevices', label: 'Medical Devices' },
            { value: 'BabyClothing', label: 'Baby Clothing' },
            { value: 'Toys', label: 'Toys' },
            { value: 'BabyGear', label: 'Baby Gear' },
            { value: 'KidsElectronics', label: 'Kids Electronics' },
            { value: 'Collectibles', label: 'Collectibles' },
            { value: 'Antiques', label: 'Antiques' },
            { value: 'Art', label: 'Art' },
            { value: 'Crafts', label: 'Crafts' },
            { value: 'OfficeSupplies', label: 'Office Supplies' },
            { value: 'IndustrialEquipment', label: 'Industrial Equipment' },
            { value: 'BusinessEquipment', label: 'Business Equipment' },
            { value: 'Other', label: 'Other' }
        ];
    }

    /**
     * Format timestamp to readable date
     * @param {number} timestamp - Unix timestamp
     * @returns {string} Formatted date string
     */
    formatDate(timestamp) {
        return new Date(timestamp * 1000).toLocaleDateString();
    }

    /**
     * Format timestamp to readable date and time
     * @param {number} timestamp - Unix timestamp
     * @returns {string} Formatted date and time string
     */
    formatDateTime(timestamp) {
        return new Date(timestamp * 1000).toLocaleString();
    }

    /**
     * Calculate time ago from timestamp
     * @param {number} timestamp - Unix timestamp
     * @returns {string} Time ago string (e.g., "2 hours ago")
     */
    timeAgo(timestamp) {
        const now = Date.now();
        const time = timestamp * 1000;
        const diff = now - time;

        const minute = 60 * 1000;
        const hour = minute * 60;
        const day = hour * 24;
        const week = day * 7;
        const month = day * 30;
        const year = day * 365;

        if (diff < minute) {
            return 'just now';
        } else if (diff < hour) {
            const minutes = Math.floor(diff / minute);
            return `${minutes} minute${minutes > 1 ? 's' : ''} ago`;
        } else if (diff < day) {
            const hours = Math.floor(diff / hour);
            return `${hours} hour${hours > 1 ? 's' : ''} ago`;
        } else if (diff < week) {
            const days = Math.floor(diff / day);
            return `${days} day${days > 1 ? 's' : ''} ago`;
        } else if (diff < month) {
            const weeks = Math.floor(diff / week);
            return `${weeks} week${weeks > 1 ? 's' : ''} ago`;
        } else if (diff < year) {
            const months = Math.floor(diff / month);
            return `${months} month${months > 1 ? 's' : ''} ago`;
        } else {
            const years = Math.floor(diff / year);
            return `${years} year${years > 1 ? 's' : ''} ago`;
        }
    }
}

// Create default API instance for development
const API = new GoodsPointAPI('');

// Export for different module systems
export { GoodsPointAPI };
export default API;

/**
 * Usage Examples:
 * 
 * // Initialize the API client
 * const api = new GoodsPointAPI('https://api.goodspoint.tech');
 * 
 * // Register a new user
 * try {
 *   const result = await api.register({
 *     username: 'johndoe',
 *     email: 'john@example.com',
 *     password: 'securepassword123'
 *   });
 *   console.log('Registration successful:', result);
 * } catch (error) {
 *   console.error('Registration failed:', error.message);
 * }
 * 
 * // Login user
 * try {
 *   await api.login({
 *     username: 'johndoe', // or use email instead
 *     password: 'securepassword123'
 *   });
 *   console.log('Login successful');
 * } catch (error) {
 *   console.error('Login failed:', error.message);
 * }
 * 
 * // Create a product
 * try {
 *   const product = await api.createProduct({
 *     title: 'iPhone 15 Pro Max',
 *     description: 'Brand new iPhone 15 Pro Max in excellent condition',
 *     product_type: 'new',
 *     purchase_type: 'buy_now',
 *     category: 'Smartphones',
 *     tags: ['iphone', 'apple', 'smartphone'],
 *     quantity: { min_quantity: 1, max_quantity: 5 },
 *     price: 1299.99
 *   });
 *   console.log('Product created:', product);
 * } catch (error) {
 *   console.error('Failed to create product:', error.message);
 * }
 * 
 * // Search products with text
 * try {
 *   const results = await api.searchProducts({
 *     query: 'iPhone 15 Pro',
 *     limit: 20
 *   });
 *   console.log('Search results:', results.results);
 * } catch (error) {
 *   console.error('Search failed:', error.message);
 * }
 * 
 * // Search products with images
 * const fileInput = document.getElementById('image-input');
 * const images = Array.from(fileInput.files);
 * try {
 *   // Validate images first
 *   for (const image of images) {
 *     const validation = api.validateSearchImage(image);
 *     if (!validation.valid) {
 *       alert(validation.error);
 *       return;
 *     }
 *   }
 *   
 *   const results = await api.searchProducts({
 *     query: 'smartphone', // optional text query
 *     limit: 10
 *   }, images);
 *   console.log('Image search results:', results.results);
 * } catch (error) {
 *   console.error('Image search failed:', error.message);
 * }
 * 
 * // Send a text message
 * try {
 *   const message = await api.sendTextMessage('user123', 'Hello, is this product still available?');
 *   console.log('Message sent:', message);
 * } catch (error) {
 *   console.error('Failed to send message:', error.message);
 * }
 * 
 * // Upload attachment
 * const fileInput = document.getElementById('file-input');
 * const file = fileInput.files[0];
 * if (file) {
 *   try {
 *     const validation = api.validateFile(file);
 *     if (!validation.valid) {
 *       alert(validation.error);
 *       return;
 *     }
 *     
 *     const message = await api.sendAttachmentMessage('user123', file);
 *     console.log('Attachment sent:', message);
 *   } catch (error) {
 *     console.error('Failed to send attachment:', error.message);
 *   }
 * }
 * 
 * // Get conversations
 * try {
 *   const conversations = await api.getConversations();
 *   console.log('Conversations:', conversations.conversations);
 * } catch (error) {
 *   console.error('Failed to get conversations:', error.message);
 * }
 * 
 * // Generate product questions with AI
 * try {
 *   const questions = await api.generateProductQuestions(
 *     'product123', 
 *     'I need to know the color and size preferences for this clothing item'
 *   );
 *   console.log('Generated questions:', questions.questions);
 * } catch (error) {
 *   console.error('Failed to generate questions:', error.message);
 * }
 * 
 * // List orders (as buyer)
 * try {
 *   const orders = await api.listOrders({ limit: 10, offset: 0 });
 *   console.log('Orders:', orders);
 * } catch (error) {
 *   console.error('Failed to list orders:', error.message);
 * }
 * 
 * // Upload gallery with progress tracking
 * const galleryFiles = Array.from(document.getElementById('gallery-input').files);
 * try {
 *   const result = await api.uploadWithProgress(
 *     '/seller/products/product123/gallery/add',
 *     galleryFiles,
 *     (progress) => {
 *       console.log(`Upload progress: ${progress}%`);
 *       document.getElementById('progress-bar').style.width = `${progress}%`;
 *     }
 *   );
 *   console.log('Gallery uploaded:', result);
 * } catch (error) {
 *   console.error('Gallery upload failed:', error.message);
 * }
 *
 * 
 * // Get WhatsApp verification status
 * try {
 *   const status = await api.getWhatsAppStatus();
 *   console.log('WhatsApp verified:', status.whatsapp_verified);
 *   console.log('WhatsApp number:', status.whatsapp_number);
 * } catch (error) {
 *   console.error('Failed to get WhatsApp status:', error.message);
 * }
 *
 * // Get personalized recommendations
 * try {
 *   const recommendations = await api.getRecommendations();
 *   console.log('Recommendations:', recommendations.rows);
 * } catch (error) {
 *   console.error('Failed to get recommendations:', error.message);
 * }
 * 
 * // Utility functions
 * console.log('Categories:', api.getProductCategories());
 * console.log('File size:', api.formatFileSize(1024 * 1024)); // "1 MB"
 * console.log('Time ago:', api.timeAgo(Date.now() / 1000 - 3600)); // "1 hour ago"
 * console.log('Category name:', api.getCategoryDisplayName('Smartphones')); // "Smartphones"
 */
