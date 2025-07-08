import { useEffect, useRef } from 'react';
import { useLocation } from 'react-router-dom';
import SearchChat from './SearchChat';
import './App.css';

export default function SearchOverlay({ isOpen, onClose, voiceMode = false, onProductNavigation }) {
  const overlayRef = useRef(null);
  const location = useLocation();
  const previousLocationRef = useRef(location.pathname);

  // Handle escape key to close overlay
  useEffect(() => {
    const handleEscape = (e) => {
      if (e.key === 'Escape' && isOpen) {
        onClose();
      }
    };

    // Prevent background scrolling only
    const preventBackgroundScroll = (e) => {
      // Only prevent if the scroll is not within the chat window
      const chatWindow = document.querySelector('.search-overlay .chat-window');
      if (chatWindow && !chatWindow.contains(e.target)) {
        e.preventDefault();
        e.stopPropagation();
      }
    };

    if (isOpen) {
      document.addEventListener('keydown', handleEscape);
      // Prevent body scroll when overlay is open
      document.body.style.overflow = 'hidden';
      
      // Prevent wheel events on the overlay background from reaching background
      document.addEventListener('wheel', preventBackgroundScroll, { passive: false });
      document.addEventListener('touchmove', preventBackgroundScroll, { passive: false });
    }

    return () => {
      document.removeEventListener('keydown', handleEscape);
      document.body.style.overflow = 'unset';
      document.removeEventListener('wheel', preventBackgroundScroll);
      document.removeEventListener('touchmove', preventBackgroundScroll);
    };
  }, [isOpen, onClose]);

  // Handle navigation to product pages
  useEffect(() => {
    const previousPath = previousLocationRef.current;
    const currentPath = location.pathname;
    
    // Only close overlay when actually navigating TO a product page from somewhere else
    // Don't close if we're already on a product page and the overlay is being opened
    if (isOpen && 
        currentPath.startsWith('/search/product/') && 
        previousPath !== currentPath && 
        !previousPath.startsWith('/search/product/')) {
      if (onProductNavigation) {
        onProductNavigation({ isOpen: true, voiceMode });
      }
      onClose();
    }
    // Close overlay when navigating to auth pages
    else if (isOpen && (currentPath === '/login' || currentPath === '/signup')) {
      onClose();
    }
    
    // Update the previous location
    previousLocationRef.current = currentPath;
  }, [location.pathname, isOpen, onClose, voiceMode, onProductNavigation]);

  // Handle outside click to close overlay
  const handleOverlayClick = (e) => {
    if (e.target === overlayRef.current) {
      onClose();
    }
  };

  // Prevent scroll events from reaching the background
  const handleOverlayScroll = (e) => {
    e.stopPropagation();
  };

  if (!isOpen) return null;

  return (
    <div 
      ref={overlayRef}
      className="search-overlay"
      onClick={handleOverlayClick}
      onScroll={handleOverlayScroll}
    >
      <div className="search-overlay-content">
        <SearchChat 
          voiceMode={voiceMode}
          onClose={onClose}
        />
      </div>
    </div>
  );
} 