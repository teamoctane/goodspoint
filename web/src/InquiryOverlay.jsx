import { useState, useEffect, useRef } from 'react';
import InquiryChat from './InquiryChat';
import { T } from './i18n';
import './App.css';

export default function InquiryOverlay({ isOpen, onClose, productId, productTitle }) {
  const [voiceMode, setVoiceMode] = useState(false);
  const [showModeSelection, setShowModeSelection] = useState(true);
  const overlayRef = useRef(null);

  // Handle escape key and prevent background scrolling
  useEffect(() => {
    const handleEscape = (e) => {
      if (e.key === 'Escape' && isOpen) {
        onClose();
      }
    };

    // Prevent background scrolling
    const preventBackgroundScroll = (e) => {
      // Only prevent if the scroll is not within the inquiry overlay
      const inquiryOverlay = document.querySelector('.inquiry-overlay');
      if (inquiryOverlay && !inquiryOverlay.contains(e.target)) {
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

  // Handle outside click to close overlay
  const handleOverlayClick = (e) => {
    if (e.target === overlayRef.current) {
      onClose();
    }
  };

  if (!isOpen) return null;

  const handleModeSelect = (isVoice) => {
    setVoiceMode(isVoice);
    setShowModeSelection(false);
  };

  return (
    <div 
      ref={overlayRef}
      className="inquiry-overlay"
      onClick={handleOverlayClick}
    >
      <div className="inquiry-overlay-content">
        {showModeSelection ? (
          <div className="mode-selection">
            <h2>How would you like to inquire about this product?</h2>
            <div className="mode-buttons">
              <button 
                className="mode-btn voice-btn"
                onClick={() => handleModeSelect(true)}
              >
                <span className="material-symbols-outlined">mic</span>
                <span>Voice</span>
              </button>
              <button 
                className="mode-btn type-btn"
                onClick={() => handleModeSelect(false)}
              >
                <span className="material-symbols-outlined">keyboard</span>
                <span>Type</span>
              </button>
            </div>
            <button className="close-btn" onClick={onClose}>
              {'Back'}
            </button>
          </div>
        ) : (
          <InquiryChat 
            voiceMode={voiceMode} 
            onClose={() => {
              setShowModeSelection(true);
              setVoiceMode(false);
              onClose();
            }}
            productId={productId}
            productTitle={productTitle}
          />
        )}
      </div>
    </div>
  );
}
