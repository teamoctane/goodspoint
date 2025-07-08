import { useEffect, useRef, useState } from 'react';
import './App.css';
import { T } from './i18n';

export default function ImageSearchOverlay({ isOpen, onClose, onProductsFound }) {
  const overlayRef = useRef(null);
  const fileInputRef = useRef(null);
  const [selectedImages, setSelectedImages] = useState([]);
  const [isDragging, setIsDragging] = useState(false);
  const [isSearching, setIsSearching] = useState(false);
  const [error, setError] = useState('');

  // Clean up and close overlay
  const handleClose = () => {
    // Clean up blob URLs
    selectedImages.forEach(img => URL.revokeObjectURL(img.url));
    // Clear selected images
    setSelectedImages([]);
    // Clear any errors
    setError('');
    // Close overlay
    onClose();
  };

  // Handle escape key to close overlay
  useEffect(() => {
    const handleEscape = (e) => {
      if (e.key === 'Escape' && isOpen) {
        handleClose();
      }
    };

    if (isOpen) {
      document.addEventListener('keydown', handleEscape);
      document.body.style.overflow = 'hidden';
    }

    return () => {
      document.removeEventListener('keydown', handleEscape);
      document.body.style.overflow = 'unset';
    };
  }, [isOpen, onClose]);

  // Handle outside click to close overlay
  const handleOverlayClick = (e) => {
    if (e.target === overlayRef.current) {
      handleClose();
    }
  };

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
    setIsDragging(true);
  };

  const handleDragLeave = (e) => {
    e.preventDefault();
    setIsDragging(false);
  };

  const handleDrop = (e) => {
    e.preventDefault();
    setIsDragging(false);
    const files = Array.from(e.dataTransfer.files);
    handleFiles(files);
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

    // Combine with existing images, but limit total to 2
    const totalImages = selectedImages.length + imageFiles.length;
    let filesToAdd = imageFiles;
    
    if (totalImages > 2) {
      const remainingSlots = 2 - selectedImages.length;
      if (remainingSlots <= 0) {
        setError('Maximum 2 images allowed. Please remove existing images first.');
        return;
      }
      filesToAdd = imageFiles.slice(0, remainingSlots);
      setError(`Only ${remainingSlots} more image${remainingSlots > 1 ? 's' : ''} can be added (maximum 2 total)`);
    }

    // Check file size (5MB limit)
    const oversizedFiles = filesToAdd.filter(file => file.size > 5 * 1024 * 1024);
    if (oversizedFiles.length > 0) {
      setError('Each image must be less than 5MB');
      return;
    }

    // Create preview URLs for new images
    const newImageData = filesToAdd.map(file => ({
      file,
      url: URL.createObjectURL(file),
      name: file.name
    }));

    // Add to existing images
    setSelectedImages(prev => [...prev, ...newImageData]);
  };

  // Remove image
  const removeImage = (index) => {
    const newImages = [...selectedImages];
    URL.revokeObjectURL(newImages[index].url);
    newImages.splice(index, 1);
    setSelectedImages(newImages);
  };

  // Search with images
  const handleSearch = async () => {
    if (selectedImages.length === 0) {
      setError('Please select at least one image');
      return;
    }

    setIsSearching(true);
    setError('');

    try {
      // Create FormData for multipart upload
      const formData = new FormData();
      
      // Add search parameters as JSON body
      const searchRequest = {
        query: null, // No text query for pure image search
        limit: 10,
        force_original: false
      };
      formData.append('body', JSON.stringify(searchRequest));
      
      // Add each selected image
      selectedImages.forEach((imageData) => {
        formData.append('images', imageData.file, imageData.name);
      });
      
      console.log('Sending image search request with', selectedImages.length, 'images');
      
      // Send to the real image search endpoint
      const response = await fetch('/products/search', {
        method: 'POST',
        body: formData,
        credentials: 'include'
      });
      
      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }
      
      const responseData = await response.json();
      console.log('Image search response:', responseData);
      
      // Transform products to expected format
      const transformedProducts = responseData.results?.map(apiProduct => {
        if (!apiProduct) return null;
        
        return {
          productId: apiProduct.product_id,
          title: apiProduct.title,
          price: apiProduct.price || '0',
          conditionDesc: apiProduct.product_type === 'new' ? 'Brand-new in factory packaging' : 'Used - excellent condition',
          maxQty: apiProduct.quantity?.max_quantity || 1,
          thumbnail: apiProduct.thumbnail_url,
          description: apiProduct.description,
          gallery: apiProduct.gallery?.map(item => item.url) || [],
          category: apiProduct.category,
          tags: apiProduct.tags || [],
          username: apiProduct.username || 'Unknown seller'
        };
      }).filter(Boolean) || [];
      
      // Return the search results
      onProductsFound(transformedProducts);
      
      // Close the image search overlay
      handleClose();
      
    } catch (error) {
      console.error('Image search error:', error);
      setError('Image search failed. Please try again.');
    } finally {
      setIsSearching(false);
    }
  };

  // Clean up URLs on unmount
  useEffect(() => {
    return () => {
      // Only clean up on unmount, not on close (handled by handleClose)
      selectedImages.forEach(img => URL.revokeObjectURL(img.url));
    };
  }, []);

  if (!isOpen) return null;

  return (
    <div 
      ref={overlayRef}
      className="search-overlay"
      onClick={handleOverlayClick}
    >
      <div className="search-overlay-content">
        <div className="image-search-container">
          <button className="back-btn" onClick={handleClose}>
            {T('back')}
          </button>
          
          <div className="image-search-header">
            <h2>{T('upload_images')}</h2>
            <p>{T('max_2_images')}</p>
          </div>

          {error && (
            <div className="error-message">
              {error}
            </div>
          )}

          <div 
            className={`image-drop-zone ${isDragging ? 'dragging' : ''}`}
            onDragOver={handleDragOver}
            onDragLeave={handleDragLeave}
            onDrop={handleDrop}
          >
            <div className="drop-zone-content">
              <span className="material-symbols-outlined">cloud_upload</span>
              <p>{T('drag_drop_images')}</p>
              <p className="file-limits">{T('max_2_images')}</p>
              <input
                type="file"
                accept="image/*"
                multiple
                onChange={handleFileChange}
                className="file-input"
              />
            </div>
          </div>

          {selectedImages.length > 0 && (
            <div className="selected-images">
              <h3>{T('selected_images')} ({selectedImages.length}/2)</h3>
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
                    >
                      <span className="material-symbols-outlined">close</span>
                    </button>
                  </div>
                ))}
              </div>
              {selectedImages.length < 2 && (
                <div className="add-more-images">
                  <button 
                    className="btn secondary"
                    onClick={() => fileInputRef.current?.click()}
                  >
                    {T('add_more_images')} ({2 - selectedImages.length} {T('remaining')})
                  </button>
                  <input
                    ref={fileInputRef}
                    type="file"
                    accept="image/*"
                    multiple
                    onChange={handleFileChange}
                    style={{ display: 'none' }}
                  />
                </div>
              )}
            </div>
          )}

          <div className="image-search-actions">
            <button 
              className="btn secondary" 
              onClick={handleClose}
              disabled={isSearching}
            >
              {T('cancel')}
            </button>
            <button 
              className="btn primary" 
              onClick={handleSearch}
              disabled={selectedImages.length === 0 || isSearching}
            >
              {isSearching ? T('searching') : T('search_products')}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
