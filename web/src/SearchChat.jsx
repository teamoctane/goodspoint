import { useEffect, useRef, useState, useCallback } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { TextInputBar, VoiceInputBar } from './SearchChatInput';
import './App.css';
import { T, useLang } from './i18n';

const STORAGE = 'wizard-chat';

// Helper function to transform API product data to our expected format
const transformProduct = (apiProduct) => {
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
};

// API wrapper for search functionality
const api = {
  // Search products using the backend API
  search: async ({ query, limit = 10, offset = 0 }) => {
    try {
      console.log('Searching with query:', query, 'limit:', limit, 'offset:', offset);
      
      // The backend expects a POST request to /products/search with multipart form data
      const formData = new FormData();
      formData.append('body', JSON.stringify({ query, limit }));
      
      const response = await fetch('/products/search', {
        method: 'POST',
        body: formData,
      });
      
      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }
      
      const responseData = await response.json();
      console.log('Search response:', responseData);
      
      const transformedProducts = responseData.results?.map(transformProduct).filter(Boolean) || [];
      
      return {
        products: transformedProducts,
        total_count: responseData.total_count || 0,
        query_enhanced: responseData.query_enhanced,
        processing_time_ms: responseData.processing_time_ms,
        cursor: offset + transformedProducts.length < (responseData.total_count || 0) ? offset + transformedProducts.length : null,
      };
    } catch (error) {
      console.error('Error searching products:', error);
      return {
        products: [],
        total_count: 0,
        cursor: null,
      };
    }
  },


};

function ProductCard({ data, onClose, ...rest }) {
  const navigate = useNavigate();
  if (!data) return null;
  
  // Get translated text during render, not in event handlers
  const inquireText = T('inquire');
  const buyNowText = T('buy_now');
  const boughtForText = T('bought_for');
  
  const handleProductClick = () => {
    // Close the search overlay before navigating
    if (onClose) onClose();
    navigate(`/product/${data.productId}`);
  };
  
  const handleBuyClick = (e) => {
    e.stopPropagation();
    alert(`${boughtForText} ₹${data.price || 0}`);
  };
  
  const handleInquireClick = (e) => {
    e.stopPropagation();
    // Close the search overlay before navigating
    if (onClose) onClose();
    // Navigate to product page with inquiry trigger
    navigate(`/product/${data.productId}`, { state: { openInquiry: true } });
  };
  
  return (
    <div className="product chat-product" {...rest} onClick={handleProductClick} style={{ cursor: 'pointer', position: 'relative' }}>
      <div className="chat-prod-img" style={{ backgroundImage: `url(${data.thumbnail})` }} />
      <div className="chat-prod-info">
        <div className="chat-prod-title">{data.title}</div>
        <div className="chat-prod-meta">
          <span className="chat-prod-price">₹{data.price || 0}</span>
          <span className="chat-prod-cond">{data.conditionDesc}</span>
        </div>
        <div className="chat-prod-desc">{data.description}</div>
        <div className="chat-prod-actions">
          <button className="pill outline action" onClick={handleBuyClick}>
            {buyNowText} ₹{data.price || 0}
          </button>
          <button className="pill outline action" onClick={handleInquireClick}>
            {inquireText}
          </button>
        </div>
      </div>
    </div>
  );
}

function SkeletonProductCard() {
  return (
    <div className="product chat-product skeleton">
      <div className="chat-prod-img skeleton-bg" />
      <div className="chat-prod-info">
        <div className="chat-prod-title skeleton-bg" style={{ width: '60%', height: 24, marginBottom: 8 }} />
        <div className="chat-prod-meta">
          <span className="chat-prod-price skeleton-bg" style={{ width: 80, height: 18 }} />
          <span className="chat-prod-cond skeleton-bg" style={{ width: 120, height: 18 }} />
        </div>
        <div className="chat-prod-desc skeleton-bg" style={{ width: '100%', height: 32, margin: '8px 0' }} />
        <div className="chat-prod-actions">
          <span className="pill outline action skeleton-bg" style={{ width: 100, height: 32 }} />
        </div>
      </div>
    </div>
  );
}

export default function SearchChat({ voiceMode: propVoiceMode, onClose }) {
  const navigate = useNavigate();
  const location = useLocation();
  useLang(); // Keep for language functionality even if not directly used
  const scrollRef = useRef(null);
  const sentinelRef = useRef(null);
  const firstProductRef = useRef(null);
  const inputRef = useRef(null);
  const bottomRef = useRef(null);
  const feedRef = useRef(null);
  const awaiting = useRef(false);
  const inputStartRef = useRef();
  const voiceInputRef = useRef();
  const conversationIdRef = useRef(null);
  
  // Generate a conversation ID if we don't have one
  useEffect(() => {
    if (!conversationIdRef.current) {
      conversationIdRef.current = `conv_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
    }
  }, []);
  
  // Get voice mode from navigation state or prop
  const voiceMode = propVoiceMode || location.state?.voiceMode === true;
  const [draft, setDraft] = useState('');

  // Get initial query from sessionStorage (set by Navbar before navigating)
  const initialQuery = sessionStorage.getItem('initial-search-query') || '';

  const [messages, setMsgs] = useState(() => {
    const saved = sessionStorage.getItem(STORAGE);
    if (saved) return JSON.parse(saved);
    if (initialQuery) {
      sessionStorage.removeItem('initial-search-query');
      return [{ role: 'user', text: initialQuery }];
    }
    return [];
  });
  const msgsRef = useRef(messages);

  useEffect(() => { msgsRef.current = messages; }, [messages]);
  const [cursor, setCursor] = useState(null);
  const [products, setProds] = useState(() => {
    const saved = sessionStorage.getItem('chat-products');
    return saved ? JSON.parse(saved) : [];
  });
  const [end, setEnd] = useState(false);
  const [loading, setLoading] = useState(false);
  const [userHasScrolled, setUserHasScrolled] = useState(false);
  const [productsHidden, setProductsHidden] = useState(false);
  const [productsToHide, setProductsToHide] = useState([]);
  const [indicators, setIndicators] = useState([]);

  // Respond to first user message after reset or mount
  useEffect(() => {
    if (
      messages.length === 1 &&
      messages[0].role === 'user'
    ) {
      (async () => {
        const userQuery = messages[0].text;
        
        // Perform search
        const searchResult = await api.search({ 
          query: userQuery, 
          limit: 10, 
          offset: 0,
          conversationId: conversationIdRef.current 
        });
        
        if (searchResult.products && searchResult.products.length > 0) {
          // Show products only, no AI response
          setProds(searchResult.products);
          setCursor(searchResult.cursor);
          setEnd(!searchResult.cursor);
        }
      })();
    }
  }, [messages]);

  useEffect(() => {
    sessionStorage.setItem(STORAGE, JSON.stringify(messages));
  }, [messages]);

  // Save products to sessionStorage on change
  useEffect(() => {
    sessionStorage.setItem('chat-products', JSON.stringify(products));
  }, [products]);

  // Fetch more products for infinite scroll
  const fetchMore = useCallback((initial = false) => {
    if (end || loading) return;
    console.log('fetchMore called - initial:', initial, 'cursor:', cursor, 'loading:', loading, 'end:', end);
    setLoading(true);
    
    // Stop voice recognition when products are about to appear
    if (voiceMode && voiceInputRef.current) {
      setTimeout(() => {
        if (voiceInputRef.current && voiceInputRef.current.stop) {
          voiceInputRef.current.stop();
        }
      }, 100);
    }
    
    const currentQuery = messages.length > 0 ? messages[messages.length - 1].text : '';
    
    api.search({ 
      query: currentQuery,
      limit: 10, 
      offset: initial ? 0 : cursor || 0,
      conversationId: conversationIdRef.current 
    }).then(r => {
      console.log('API response:', r);
      if (initial) {
        setProds(r.products || []);
        setCursor(r.cursor);
        setEnd(!r.cursor);
      } else {
        setProds(p => [...p, ...(r.products || [])]);
        setCursor(r.cursor);
        setEnd(!r.cursor);
      }
      setLoading(false);
    }).catch(error => {
      console.error('Search failed:', error);
      setLoading(false);
    });
  }, [end, loading, cursor, voiceMode, messages]);

  // Enable infinite scroll immediately for seamless product loading
  useEffect(() => {
    if (!userHasScrolled) {
      console.log('Enabling infinite scroll for seamless product loading');
      setUserHasScrolled(true);
    }
  }, [userHasScrolled]);

  // IntersectionObserver for seamless infinite scroll
  useEffect(() => {
    if (!sentinelRef.current) return;
    const io = new window.IntersectionObserver(entries => {
      if (entries[0].isIntersecting && !loading && !end && products.length >= 3) {
        console.log('Sentinel triggered, loading more products seamlessly');
        fetchMore(false);
      }
    }, { root: scrollRef.current, threshold: 0.1, rootMargin: '200px' });
    io.observe(sentinelRef.current);
    return () => io.disconnect();
  }, [cursor, end, loading, fetchMore, products.length]);

  // If no messages, auto-focus the input bar for first query
  useEffect(() => {
    if (messages.length === 0 && inputRef.current) {
      inputRef.current.focus();
    }
  }, [messages.length]);

  // Auto-scroll to show first product near the top when products appear
  useEffect(() => {
    if (products.length > 0) {
      console.log('Products loaded, scrolling to show first product near top');
      setTimeout(() => {
        if (scrollRef.current) {
          const firstProduct = document.querySelector('.product.chat-product');
          if (firstProduct) {
            firstProduct.scrollIntoView({ behavior: 'smooth', block: 'start' });
          }
        }
      }, 100);
    }
  }, [products.length]);

  // Stop voice recognition when products appear
  useEffect(() => {
    if (products.length > 0 && voiceMode && voiceInputRef.current) {
      console.log('Products appeared, stopping voice recognition');
      setTimeout(() => {
        if (voiceInputRef.current && voiceInputRef.current.stop) {
          voiceInputRef.current.stop();
        }
      }, 100);
    }
  }, [products.length, voiceMode]);

  // Reset chat if a new initial-search-query is set
  useEffect(() => {
    const initialQuery = sessionStorage.getItem('initial-search-query');
    if (initialQuery) {
      sessionStorage.removeItem('initial-search-query');
      setMsgs([{ role: 'user', text: initialQuery }]);
      setProds([]);
      setCursor(null);
      setEnd(false);
      setLoading(false);
      setUserHasScrolled(false);
      sessionStorage.removeItem('chat-products');
      sessionStorage.removeItem(STORAGE);
    }
  }, [location.key]);

  // Clear all state and sessionStorage on unmount
  useEffect(() => {
    return () => {
      setMsgs([]);
      setProds([]);
      setCursor(null);
      setEnd(false);
      setLoading(false);
      sessionStorage.removeItem('chat-products');
      sessionStorage.removeItem(STORAGE);
    };
  }, []);

  // Main message handler
  const handleSend = async (text) => {
    if (!text.trim()) return;
    
    // Clear the draft immediately after sending
    setDraft('');
    
    // If products are already shown, start refinement
    if (products.length > 0) {
      // Show indicator for products being hidden BEFORE adding the user message
      const newIndicator = {
        id: Date.now(),
        messageId: messages.length, // This will be the position where the indicator appears (before the new message)
        count: products.length,
        timestamp: Date.now()
      };
      setIndicators(prev => [...prev, newIndicator]);
      
      // Add user message after creating the indicator
      setMsgs(m => [...m, { role: 'user', text }]);
      
      // Hide existing products
      setProductsToHide([...products]);
      setProductsHidden(true);
      
      // Clear products and perform new search immediately
      setProds([]);
      setProductsToHide([]);
      setCursor(null);
      setEnd(false);
      setLoading(false);
      setUserHasScrolled(false);
      
      // Perform new search with refined query
      const searchResult = await api.search({ 
        query: text, 
        limit: 10, 
        offset: 0,
        conversationId: conversationIdRef.current 
      });
      
      if (searchResult.products && searchResult.products.length > 0) {
        setProds(searchResult.products);
        setCursor(searchResult.cursor);
        setEnd(!searchResult.cursor);
      }
      
      setProductsHidden(false);
      return;
    }
    
    // Normal flow (initial chat)
    const conversation = [...msgsRef.current, { role: 'user', text }];
    setMsgs(conversation);
    msgsRef.current = conversation;

    if (awaiting.current) return;
    awaiting.current = true;

    try {
      // Perform search
      const searchResult = await api.search({ 
        query: text, 
        limit: 10, 
        offset: 0,
        conversationId: conversationIdRef.current 
      });
      
      if (searchResult.products && searchResult.products.length > 0) {
        // Show products only, no AI response
        setProds(searchResult.products);
        setCursor(searchResult.cursor);
        setEnd(!searchResult.cursor);
      }
      // No AI response even if no products found
    } catch (error) {
      console.error('Error in handleSend:', error);
      // No AI error response, just log the error
    } finally {
      awaiting.current = false;
    }
  };

  // Focus input after chat reset and after messages are added
  useEffect(() => {
    if (
      messages.length === 1 &&
      messages[0].role === 'user' &&
      inputRef.current
    ) {
      inputRef.current.focus();
    }
  }, [messages, inputRef]);

  // Focus input after any message is added
  useEffect(() => {
    if (messages.length > 0 && inputRef.current && !voiceMode) {
      setTimeout(() => {
        if (inputRef.current) {
          inputRef.current.focus();
        }
      }, 100);
    }
  }, [messages.length, voiceMode]);

  // Render
  return (
    <div className="chat-wrap">
      <button className="back-btn" onClick={() => onClose ? onClose() : navigate(-1)}>{T('back')}</button>
      <div className="chat-window" ref={scrollRef}>
        {messages.map((m, i) => (
          <div key={i} style={{ width: '100%', display: 'flex', flexDirection: 'column' }}>
            {/* Show indicators before messages that represent refinements */}
            {indicators.map(indicator => 
              indicator.messageId === i ? (
                <div key={indicator.id} className="products-hidden-indicator">
                  <span className="material-symbols-outlined">inventory_2</span>
                  {indicator.count} {T('products_shown')}
                </div>
              ) : null
            )}
            <div className={`bubble ${m.role}`}>
              {m.text}
            </div>
          </div>
        ))}
        {(products.length > 0 || productsToHide.length > 0) && (
          <div className={`product-feed ${productsHidden ? 'hiding' : ''}`} ref={feedRef}>
            {(products.length > 0 ? products : productsToHide).filter(Boolean).map((p, idx) => (
              <ProductCard 
                key={p.productId} 
                data={p} 
                onClose={onClose}
                ref={idx === 0 ? firstProductRef : undefined} 
              />
            ))}
            {!end && products.length > 0 && <div ref={sentinelRef} className="sentinel" />}
            {loading && Array.from({ length: 3 }).map((_, i) => <SkeletonProductCard key={i} />)}
          </div>
        )}
        <div ref={bottomRef} />
      </div>
      {(() => {
        const Input = voiceMode ? VoiceInputBar : TextInputBar;
        return (
          <Input
            ref={voiceMode ? voiceInputRef : undefined}
            onSend={handleSend}
            autoStart={voiceMode}
            value={draft}
            setValue={setDraft}
            getStartRef={fn => (inputStartRef.current = fn)}
            inputRef={inputRef}
          />
        );
      })()}
    </div>
  );
}


