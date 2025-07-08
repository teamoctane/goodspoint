import { useEffect, useRef } from 'react';
import { useNavigate } from 'react-router-dom';
import './App.css';
import { T } from './i18n';

function ProductCard({ data, onClose }) {
  const navigate = useNavigate();
  if (!data) return null;
  
  const buyNowText = T('buy_now');
  const inquireText = T('inquire');
  
  const handleProductClick = () => {
    if (onClose) onClose();
    navigate(`/product/${data.productId}`);
  };
  
  const handleInquireClick = (e) => {
    e.stopPropagation();
    if (onClose) onClose();
    navigate(`/product/${data.productId}`, { state: { openInquiry: true } });
  };
  
  return (
    <div 
      className="product chat-product" 
      onClick={handleProductClick} 
      style={{ cursor: 'pointer', position: 'relative' }}
    >
      <div className="chat-prod-img" style={{ backgroundImage: `url(${data.thumbnail})` }} />
      <div className="chat-prod-info">
        <div className="chat-prod-title">{data.title}</div>
        <div className="chat-prod-meta">
          <span className="chat-prod-price">₹{data.price}</span>
          <span className="chat-prod-cond">{data.conditionDesc}</span>
        </div>
        <div className="chat-prod-desc">{data.description}</div>
        <div className="chat-prod-actions">
          <button 
            className="pill outline action" 
            onClick={e => { 
              e.stopPropagation(); 
              alert(`${buyNowText} ₹${data.price}`); 
            }}
          >
            {buyNowText} ₹{data.price}
          </button>
          <button className="pill outline action" onClick={handleInquireClick}>
            {inquireText}
          </button>
        </div>
      </div>
    </div>
  );
}

export default function ProductResultsOverlay({ isOpen, onClose, products = [] }) {
  const overlayRef = useRef(null);
  const scrollRef = useRef(null);

  // Handle escape key to close overlay
  useEffect(() => {
    const handleEscape = (e) => {
      if (e.key === 'Escape' && isOpen) {
        onClose();
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
      onClose();
    }
  };

  if (!isOpen) return null;

  return (
    <div 
      ref={overlayRef}
      className="search-overlay"
      onClick={handleOverlayClick}
    >
      <div className="search-overlay-content">
        <div className="product-results-container">
          <button className="back-btn" onClick={onClose}>
            {T('back')}
          </button>
          
          <div className="product-results-header">
            <h2>{T('search_results')}</h2>
            <p>Found {products.length} product{products.length !== 1 ? 's' : ''} matching your images</p>
          </div>

          <div className="product-results-content" ref={scrollRef}>
            {products.length > 0 ? (
              <div className="product-results-grid">
                {products.map((product) => (
                  <ProductCard 
                    key={product.productId}
                    data={product}
                    onClose={onClose}
                  />
                ))}
              </div>
            ) : (
              <div className="no-results">
                <span className="material-symbols-outlined">search_off</span>
                <h3>{T('no_products_found')}</h3>
                <p>{T('try_different_images')}</p>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
