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
      // For now, use the same API as regular search since there's no image search endpoint yet
      // In the future, this would be replaced with an actual image search API
      const response = await fetch('/products/f613abcd-36e7-44f0-9df6-db6660e5df75', {
        credentials: 'include'
      });
      
      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }
      
      const response_data = await response.json();
      const product = response_data.product;
      
      if (product) {
        const transformedProduct = {
          productId: product.product_id,
          title: product.title,
          price: 0,
          conditionDesc: product.product_type === 'new' ? 'Brand-new in factory packaging' : 'Used - excellent condition',
          maxQty: product.quantity?.max_quantity || 1,
          thumbnail: product.thumbnail_url,
          description: product.description,
          gallery: product.gallery?.map(item => item.url) || [],
          category: product.category,
          tags: product.tags || []
        };
        
        // Return the real product data
        onProductsFound([transformedProduct]);
      } else {
        onProductsFound([]);
      }
      
      // Close the image search overlay
      handleClose();
      
    } catch (error) {
      console.error('Image search error:', error);
      setError('Search failed. Please try again.');
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
            {'Back'}
          </button>
          
          <div className="image-search-header">
            <h2>{'Upload Image'}</h2>
            <p>{'Maximum 2 images, 5MB each'}</p>
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
              <p>{'Drag and drop images here or click to select'}</p>
              <p className="file-limits">{'Maximum 2 images, 5MB each'}</p>
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
              <h3>{'Selected Images'} ({selectedImages.length}/2)</h3>
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
                    {'Add More Images'} ({2 - selectedImages.length} {'remaining'})
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
              {'Cancel'}
            </button>
            <button 
              className="btn primary" 
              onClick={handleSearch}
              disabled={selectedImages.length === 0 || isSearching}
            >
              {isSearching ? 'Searching...' : 'Search Products'}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
