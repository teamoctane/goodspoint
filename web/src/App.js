import { useEffect, useRef, useState } from 'react';
import {
  Routes,
  Route,
  useNavigate,
  useLocation,
  useParams,
} from 'react-router-dom';
import { T, useLang, LANGS } from './i18n';
import './App.css';
import SearchOverlay from './SearchOverlay';
import InquiryOverlay from './InquiryOverlay';
import ImageSearchOverlay from './ImageSearchOverlay';
import ProductResultsOverlay from './ProductResultsOverlay';
import Chat from './Chat';
import LoadingIndicator from './LoadingIndicator';

/* ───────────────── dummy catalogue & mini-API ───────────────── */
// const getDummyData = (lang) => {
//   const isHindi = lang === LANGS.HI;
//   
//   return Array.from({ length: 15 }, (_, i) => ({
//     productId: `P${i + 1}`,
//     title: isHindi 
//       ? `औद्योगिक विजेट मॉडल ${i + 1}`
//       : `Industrial Widget Model ${i + 1}`,
//     price: (i + 1) * 15,
//     conditionDesc: isHindi
//       ? (i % 2 ? 'हल्का प्रयोग किया गया – उत्कृष्ट कार्य स्थिति' : 'फैक्टरी पैकेजिंग में ब्रांड-न्यू')
//       : (i % 2 ? 'Lightly used – excellent working order' : 'Brand-new in factory packaging'),
//     maxQty: (i + 1) * 3,
//     thumbnail: `https://picsum.photos/seed/thumb${i}/600/400`,
//     description: isHindi
//       ? `मॉडल ${i + 1} विजेट भारी-शुल्क औद्योगिक वर्कफ्लो के लिए इंजीनियर किया गया। ` +
//         `प्रबलित मिश्र धातु चेसिस, अनुकूली टॉर्क मॉड्यूलेशन और IoT टेलीमेट्री।`
//       : `Model ${i + 1} widget engineered for heavy-duty industrial workflows. ` +
//         `Reinforced alloy chassis, adaptive torque modulation and IoT telemetry.`,
//     gallery: [
//       `https://picsum.photos/seed/g${i}a/600/400`,
//       `https://picsum.photos/seed/g${i}b/600/400`,
//       `https://picsum.photos/seed/g${i}c/600/400`,
//     ],
//   }));
// };

// Helper function to transform API product data to our expected format
const transformProduct = (apiProduct) => {
  if (!apiProduct) return null;
  
  return {
    productId: apiProduct.product_id,
    title: apiProduct.title,
    price: 0, // Price not provided in API, defaulting to 0
    conditionDesc: apiProduct.product_type === 'new' ? 'Brand-new in factory packaging' : 'Used - excellent condition',
    maxQty: apiProduct.quantity?.max_quantity || 1,
    thumbnail: apiProduct.thumbnail_url,
    description: apiProduct.description,
    gallery: apiProduct.gallery?.map(item => item.url) || [],
    category: apiProduct.category,
    tags: apiProduct.tags || []
  };
};

const api = {
  // Helper function to refresh authentication token
  refreshAuth: async () => {
    try {
      console.log('Attempting to refresh authentication...');
      const response = await fetch('/auth/login', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json'
        },
        credentials: 'include',
        body: JSON.stringify({
          username: 'testuser',
          password: 'TestPass123!'
        })
      });
      
      if (response.ok) {
        // Clear any old token since we're using cookie-based auth
        localStorage.removeItem('GOODSPOINT_AUTHENTICATION');
        console.log('Authentication refreshed successfully');
        return true;
      } else {
        console.error('Failed to refresh authentication');
        return false;
      }
    } catch (error) {
      console.error('Error refreshing authentication:', error);
      return false;
    }
  },

  // Helper function to get authentication headers
  getAuthHeaders: () => {
    return {
      'Content-Type': 'application/json',
      // Using credentials: 'include' instead of manual cookies
    };
  },
  
  search: async ({ limit, cursor, lang }) => { // eslint-disable-line no-unused-vars
    try {
      // For now, just fetch all products since there's only one
      // Later you can add pagination and search parameters
      // Note: limit, cursor, lang parameters will be used when API supports them
      const response = await fetch('/products/f613abcd-36e7-44f0-9df6-db6660e5df75', {
        headers: api.getAuthHeaders(),
        credentials: 'include'
      });
      
      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }
      
      const response_data = await response.json();
      const product = response_data.product; // Extract product from wrapper
      const transformedProduct = transformProduct(product);
      
      return {
        products: transformedProduct ? [transformedProduct] : [],
        cursor: null, // No pagination for single product
      };
    } catch (error) {
      console.error('Error fetching products:', error);
      return {
        products: [],
        cursor: null,
      };
    }
  },
  
  get: async (id, lang) => { // eslint-disable-line no-unused-vars
    try {
      // Note: lang parameter will be used when API supports localization
      const response = await fetch(`/products/${id}`, {
        headers: api.getAuthHeaders(),
        credentials: 'include'
      });
      
      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }
      
      const response_data = await response.json();
      const product = response_data.product; // Extract product from wrapper
      return transformProduct(product);
    } catch (error) {
      console.error('Error fetching product:', error);
      return null;
    }
  },

  create: async (formData) => {
    const makeRequest = async () => {
      console.log('Creating product...');
      
      return await fetch('/seller/products/create', {
        method: 'POST',
        credentials: 'include', // Important for cookie-based auth
        body: formData
      });
    };

    try {
      let response = await makeRequest();
      console.log('Response status:', response.status);
      
      // If we get 401, try to refresh auth and retry once
      if (response.status === 401) {
        console.log('Got 401, attempting to refresh authentication...');
        const refreshed = await api.refreshAuth();
        if (refreshed) {
          console.log('Auth refreshed, retrying request...');
          response = await makeRequest();
          console.log('Retry response status:', response.status);
        }
      }
      
      if (!response.ok) {
        const errorText = await response.text();
        console.error('Error response:', errorText);
        let errorData;
        try {
          errorData = JSON.parse(errorText);
        } catch {
          errorData = { message: errorText };
        }
        throw new Error(errorData.message || `HTTP error! status: ${response.status}`);
      }
      
      const responseData = await response.json();
      console.log('Success response:', responseData);
      return responseData;
    } catch (error) {
      console.error('Error creating product:', error);
      throw error;
    }
  },

  // Get current user information
  getCurrentUser: async () => {
    try {
      const response = await fetch('/auth/user', {
        credentials: 'include'
      });
      
      if (response.ok) {
        const data = await response.json();
        return data.user;
      } else {
        console.error('Failed to get current user:', response.status);
        return null;
      }
    } catch (error) {
      console.error('Error getting current user:', error);
      return null;
    }
  },

  // Logout user
  logout: async () => {
    try {
      const response = await fetch('/auth/logout', {
        method: 'POST',
        credentials: 'include'
      });
      
      if (response.ok) {
        localStorage.removeItem('logged');
        localStorage.removeItem('GOODSPOINT_AUTHENTICATION');
        return true;
      } else {
        console.error('Failed to logout:', response.status);
        return false;
      }
    } catch (error) {
      console.error('Error during logout:', error);
      return false;
    }
  },
};

/* ───────── NAVBAR ───────── (unchanged) */
function Navbar({ onSearchOpen, onImageSearchOpen }) {
  const [locked, setLocked] = useState(null);
  const [hover,  setHover]  = useState(null);
  const [drawer, setDrawer] = useState(false);
  const [logged, setLogged] = useState(false);
  const { lang, toggle } = useLang();

  const navRef    = useRef(null);
  const burgerRef = useRef(null);
  const drawerRef = useRef(null);
  const inputRef  = useRef(null);

  const navigate  = useNavigate();
  const location  = useLocation();

  // Check login state from localStorage
  useEffect(() => {
    const isLogged = localStorage.getItem('logged') === 'true';
    setLogged(isLogged);
  }, []);

  // Listen for storage changes to update login state
  useEffect(() => {
    const sync = (e) => {
      if (e.key === 'logged') {
        setLogged(e.newValue === 'true');
      }
    };
    window.addEventListener('storage', sync);
    return () => window.removeEventListener('storage', sync);
  }, []);

  /* outside-click handling */
  useEffect(() => {
    const close = (e) => {
      if (
        drawer &&
        !drawerRef.current.contains(e.target) &&
        !burgerRef.current.contains(e.target)
      ) setDrawer(false);
      if (locked && !navRef.current.contains(e.target)) setLocked(null);
    };
    window.addEventListener('mousedown', close);
    return () => window.removeEventListener('mousedown', close);
  }, [drawer, locked]);

  useEffect(() => setDrawer(false), [location.pathname]);

  useEffect(() => {
    if (locked === 'type' && inputRef.current) inputRef.current.focus();
  }, [locked]);

  const pillCls = (id) => {
    if (locked === 'type')
      return id === 'type' ? 'textbox centre glow type' : 'circle';
    if (locked === 'speak')
      return id === 'speak' ? 'textbox centre glow speak' : 'circle';
    if (id === 'speak' && hover == null) return 'pill centre';
    if (hover === id) return 'pill preview';
    return 'circle';
  };

  const launchSearch = () => {
    sessionStorage.removeItem('wizard-state');
    sessionStorage.removeItem('wizard-chat'); // Reset chat state on new search
    // Save the input value as initial-search-query for chat
    const input = inputRef.current?.value?.trim();
    if (input) sessionStorage.setItem('initial-search-query', input);
    setLocked(null);
    if (onSearchOpen) {
      onSearchOpen(false); // Open search overlay in text mode
    } else {
      navigate('/search');
    }
  };

  return (
    <>
      <nav ref={navRef} className="nav">
        <button
          ref={burgerRef}
          className={`circle burger${drawer ? ' open' : ''}`}
          onClick={() => setDrawer(!drawer)}
        >
          <span className="material-symbols-outlined">
            {drawer ? 'close' : 'menu'}
          </span>
        </button>

        <img
          src="/logodark.svg"
          alt="GoodsPoint"
          className="logo"
          style={{ cursor: 'pointer' }}
          onClick={() => navigate('/')}
        />

        <div className="mid">
          {['speak', 'type', 'image'].map((id) => {
            const icon = { speak:'mic', type:'keyboard', image:'image_search' }[id];
            const txt  = { speak:T('speak'), type:T('type'), image:T('upload_image') }[id];
            const cls  = pillCls(id);

            return (
              <div
                key={id}
                className={cls}
                role="button"
                onMouseEnter={() => setHover(id)}
                onMouseLeave={() => setHover(null)}
                onClick={() => {
                  if (id === 'speak') {
                    sessionStorage.removeItem('wizard-chat');   // clear old chat
                    if (onSearchOpen) {
                      onSearchOpen(true); // Open search overlay in voice mode
                    } else {
                      navigate('/search', { state: { voiceMode: true } });
                    }
                    return;
                  }
                  if (id === 'type') {
                    setLocked('type');                          // keep old text behaviour
                    return;
                  }
                  if (id === 'image') {
                    sessionStorage.removeItem('wizard-chat');   // clear old chat
                    if (onImageSearchOpen) {
                      onImageSearchOpen(); // Open image search overlay
                    } else {
                      navigate('/search', { state: { imageMode: true } });
                    }
                    return;
                  }
                  setLocked(null);
                }}
              >
                <span className="material-symbols-outlined">{icon}</span>
                {cls.startsWith('pill') && <span className="txt">{txt}</span>}

                {cls.includes('textbox') && id==='type' && (
                  <>
                    <input
                      ref={inputRef}
                      className="input"
                      placeholder={T('type_placeholder')}
                      onKeyDown={(e)=>e.key==='Enter'&&launchSearch()}
                    />
                    <button className="submit" onClick={launchSearch}>
                      <span className="material-symbols-outlined">arrow_forward</span>
                    </button>
                  </>
                )}

                {cls.includes('textbox') && id==='speak' && (
                  <input readOnly className="input" placeholder={T('listening')} />
                )}
              </div>
            );
          })}
        </div>

        {logged ? (
          <button className="pill outline account" onClick={()=>navigate('/dashboard')}>
            {T('dashboard')}
          </button>
        ) : (
          <button className="pill outline login" onClick={()=>navigate('/login')}>
            {T('login')}
          </button>
        )}

        <button className="pill outline chat" onClick={()=>navigate('/chat')}>
          <span className="material-symbols-outlined">chat_bubble</span>
          <span className="txt">{T('chat')}</span>
        </button>

        {/* NEW language pill */}
        <button
          className="pill lang-btn"
          onClick={toggle}
          title={lang === LANGS.EN ? T('hindi') : T('english')}
        >
          {lang === LANGS.EN ? T('hindi') : T('english')}
        </button>
      </nav>

      <aside ref={drawerRef} className={drawer ? 'drawer open' : 'drawer'}>
        <button className="drawer-link" onClick={() => navigate('/')}>{T('home')}</button>
        <button className="drawer-link" onClick={() => navigate('/chat')}>{T('chat')}</button>
        <button className="drawer-link" onClick={() => navigate('/dashboard')}>{T('dashboard')}</button>
        <button className="drawer-link" onClick={() => navigate('/settings')}>{T('settings')}</button>
        <button className="drawer-link" onClick={() => navigate('/sell')}>{T('sell')}</button>
        <button className="drawer-link" onClick={() => navigate('/about')}>{T('about')}</button>
      </aside>
    </>
  );
}

/* ───────── AUTH ───────── */
function Auth({ mode }) {
  const navigate = useNavigate();
  const [email, setE] = useState('');
  const [username, setU] = useState('');
  const [pw, setP] = useState('');
  const [confirmPw, setConfirmPw] = useState('');
  const [show, setS] = useState(false);
  const [showConfirm, setShowConfirm] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [passwordFocused, setPasswordFocused] = useState(false);

  const submit = async (e) => {
    e.preventDefault();
    if (!pw.trim()) return;
    
    // For login: require username only
    // For signup: require both email and username
    if (mode === 'login' && !username.trim()) {
      setError('Please enter username');
      return;
    }
    if (mode === 'signup' && (!email.trim() || !username.trim())) {
      setError('Please enter both email and username');
      return;
    }

    // For signup: check password confirmation
    if (mode === 'signup' && pw !== confirmPw) {
      setError('Passwords do not match');
      return;
    }

    setLoading(true);
    setError('');

    try {
      const endpoint = mode === 'login' ? '/auth/login' : '/auth/register';
      const payload = mode === 'login' 
        ? { 
            username: username.trim(),
            password: pw 
          }
        : { 
            email: email.trim(), 
            username: username.trim(), 
            password: pw 
          };

      const response = await fetch(endpoint, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json'
        },
        credentials: 'include',
        body: JSON.stringify(payload)
      });

      if (response.ok) {
        const data = await response.json();
        console.log(`${mode} successful:`, data);
        
        // For signup, automatically log the user in to set the authentication cookie
        if (mode === 'signup') {
          console.log('Registration successful, now logging in automatically...');
          try {
            const loginResponse = await fetch('/auth/login', {
              method: 'POST',
              headers: {
                'Content-Type': 'application/json'
              },
              credentials: 'include',
              body: JSON.stringify({
                username: username.trim(),
                password: pw
              })
            });
            
            if (loginResponse.ok) {
              console.log('Auto-login after registration successful');
            } else {
              console.error('Auto-login after registration failed');
            }
          } catch (loginError) {
            console.error('Error during auto-login after registration:', loginError);
          }
        }
        
        // Set logged state
        localStorage.setItem('logged', 'true');
        window.dispatchEvent(new StorageEvent('storage', { key: 'logged', newValue: 'true' }));
        
        // Navigate to dashboard
        navigate('/dashboard');
      } else {
        const errorData = await response.json();
        setError(errorData.error || `${mode} failed`);
      }
    } catch (error) {
      console.error(`${mode} error:`, error);
      setError('Network error. Please try again.');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="auth-wrap">
      <form className="auth-card" onSubmit={submit}>
        <h2>{mode === 'login' ? T('log_in') : T('signup')}</h2>

        {error && (
          <div style={{ 
            color: '#c33', 
            background: '#fee', 
            padding: '0.5rem', 
            borderRadius: '0.5rem',
            border: '1px solid #fcc',
            fontSize: '0.9rem'
          }}>
            {error}
          </div>
        )}

        {mode === 'signup' && (
          <input 
            type="text" 
            placeholder={T('username')} 
            value={username} 
            onChange={e => setU(e.target.value)}
            required
          />
        )}

        {mode === 'signup' && (
          <input 
            type="email" 
            placeholder={T('email')} 
            value={email} 
            onChange={e => setE(e.target.value)}
            required
          />
        )}

        {mode === 'login' && (
          <input 
            type="text" 
            placeholder={T('username')} 
            value={username} 
            onChange={e => setU(e.target.value)}
            required
          />
        )}

        <div className="password-wrap">
          <input 
            type={show ? 'text' : 'password'} 
            placeholder={T('password')}
            value={pw} 
            onChange={e => setP(e.target.value)}
            onFocus={() => setPasswordFocused(true)}
            onBlur={() => setPasswordFocused(false)}
            required
          />
          <span 
            className="material-symbols-outlined toggle-eye" 
            onClick={() => setS(!show)}
          >
            {show ? 'visibility_off' : 'visibility'}
          </span>
        </div>

        {mode === 'signup' && passwordFocused && (
          <div style={{ 
            fontSize: '0.8rem', 
            color: 'var(--dark-grey)', 
            marginTop: '-0.5rem',
            padding: '0.5rem',
            background: 'var(--light-grey)',
            borderRadius: '0.5rem',
            lineHeight: '1.4'
          }}>
            <strong>Password requirements:</strong>
            <ul style={{ margin: '0.25rem 0 0 1rem', padding: 0 }}>
              <li>At least 8 characters long</li>
              <li>Contains at least one uppercase letter</li>
              <li>Contains at least one lowercase letter</li>
              <li>Contains at least one number</li>
              <li>Contains at least one special character (!@#$%^&*)</li>
            </ul>
          </div>
        )}

        {mode === 'signup' && (
          <div className="password-wrap">
            <input 
              type={showConfirm ? 'text' : 'password'} 
              placeholder="Confirm Password"
              value={confirmPw} 
              onChange={e => setConfirmPw(e.target.value)}
              required
              style={{
                borderColor: confirmPw && pw && confirmPw !== pw ? '#ef4444' : undefined
              }}
            />
            <span 
              className="material-symbols-outlined toggle-eye" 
              onClick={() => setShowConfirm(!showConfirm)}
            >
              {showConfirm ? 'visibility_off' : 'visibility'}
            </span>
          </div>
        )}

        {mode === 'signup' && confirmPw && pw && confirmPw !== pw && (
          <div style={{ 
            fontSize: '0.8rem', 
            color: '#ef4444', 
            marginTop: '-0.5rem',
            padding: '0.5rem',
            background: '#fee',
            borderRadius: '0.5rem',
            border: '1px solid #fcc'
          }}>
            Passwords do not match
          </div>
        )}

        <button className="btn" type="submit" disabled={loading}>
          {loading 
            ? (mode === 'login' ? 'Logging in...' : 'Creating account...')
            : (mode === 'login' ? T('log_in') : T('create_account'))
          }
        </button>

        <span className="link" onClick={() => navigate(mode === 'login' ? '/signup' : '/login')}>
          {mode === 'login' ? T('need_account') : T('have_account')}
        </span>
      </form>
    </div>
  );
}

/* ╔════════════════════════════════════════╗
   ║  PRODUCT PAGE  (share & copy icon)     ║
   ╚════════════════════════════════════════╝ */
function ProductPage() {
  const { id } = useParams();
  const navigate = useNavigate();
  const location = useLocation();
  const { lang } = useLang();

  const [prod,setProd]   = useState(null);
  const [loading, setLoading] = useState(true);
  const [idx,setIdx]     = useState(0);
  const [pause,setPause] = useState(false);
  const [inquiryOverlay, setInquiryOverlay] = useState(false);

  /* icon feedback state */
  const [linkGreen,setLinkGreen]   = useState(false);
  const [shareGreen,setShareGreen] = useState(false);

  useEffect(() => {
    const fetchProduct = async () => {
      setLoading(true);
      try {
        const product = await api.get(id, lang);
        setProd(product);
        setIdx(0);
        setPause(false);
      } catch (error) {
        console.error('Failed to fetch product:', error);
      } finally {
        setLoading(false);
      }
    };
    
    fetchProduct();
  }, [id, lang]);

  // Handle opening inquiry from navigation state
  useEffect(() => {
    if (location.state?.openInquiry && prod) {
      setInquiryOverlay(true);
      // Clear the state to prevent reopening on future navigations
      navigate(location.pathname, { replace: true, state: {} });
    }
  }, [location.state, prod, navigate, location.pathname]);

  useEffect(()=>{
    if(!prod||pause) return;
    const t=setInterval(()=>setIdx(i=>(i+1)%(prod.gallery.length+1)),4000);
    return ()=>clearInterval(t);
  },[prod,pause]);

  const flash = (setter)=>{
    setter(true); setTimeout(()=>setter(false),1500);
  };

  const copy = ()=>{
    navigator.clipboard.writeText(window.location.href);
    flash(setLinkGreen);
  };

  const share = ()=>{
    const url=window.location.href;
    const data={title: prod.title, text: prod.description.slice(0,80)+'…', url};
    if(navigator.share){
      navigator.share(data).catch(()=>{});
    }else{
      navigator.clipboard.writeText(url);
      flash(setShareGreen);
    }
  };

  // Show loading indicator while fetching product
  if (loading) {
    return <LoadingIndicator />;
  }

  if(!prod) return null;
  const gallery=[prod.thumbnail,...prod.gallery];
  
  // Get translated text during render, not in event handlers
  const boughtForText = T('bought_for');
  const backText = T('back');
  const buyNowText = T('buy_now');
  const inquireText = T('inquire');
  const maxQtyText = T('max_qty');
  
  const buy = ()=>alert(`${boughtForText} ₹${prod.price}`);
  const inquire = () => setInquiryOverlay(true);

  return(
    <div className="product-page stage">
      <button className="back-btn" onClick={()=>navigate(-1)}>{backText}</button>

      <div className="prod-flex">
        {/* images */}
        <div className="left-col">
          <div className="hero-wrap">
            <img src={gallery[idx]} alt="hero" className="hero-img"/>
            <button className="circle burger pause-btn" onClick={()=>setPause(p=>!p)}>
              <span className="material-symbols-outlined">
                {pause?'play_arrow':'pause'}
              </span>
            </button>
          </div>
          <div className="gallery">
            {gallery.map((g,i)=>(
              <img key={i} src={g} alt={`g${i}`}
                onClick={()=>{setIdx(i);setPause(true);}}
                style={{border: idx===i?'2px solid var(--teal)':'1px solid var(--grey)'}}/>
            ))}
          </div>
        </div>

        {/* details */}
        <div className="right-col">
          <h1 className="hero p-title">{prod.title}</h1>
          <span className="price-large">₹{prod.price}</span>

          <div className="meta">
            <span>{prod.conditionDesc}</span>
            <span>{maxQtyText}: {prod.maxQty}</span>
          </div>

          <p className="prod-desc">{prod.description}</p>

          <div className="action-row" style={{gap:'.5rem'}}>
            <button className="pill outline action" onClick={buy}>
              {buyNowText}&nbsp;₹{prod.price}
            </button>
            <button className="pill outline action" onClick={inquire}>
              {inquireText}
            </button>

            {/* circular icon buttons */}
            <button className="circle" onClick={share}>
              <span
                className="material-symbols-outlined"
                style={{color: shareGreen ? 'var(--teal)' : undefined}}
              >
                share
              </span>
            </button>
            <button className="circle" onClick={copy}>
              <span
                className="material-symbols-outlined"
                style={{
                  color: linkGreen ? 'white' : undefined,
                  backgroundColor: linkGreen ? '#22c55e' : undefined,
                  borderRadius: linkGreen ? '50%' : undefined,
                  padding: linkGreen ? '0.25rem' : undefined
                }}
              >
                link
              </span>
            </button>
          </div>
        </div>
      </div>
      
      <InquiryOverlay 
        isOpen={inquiryOverlay}
        onClose={() => setInquiryOverlay(false)}
        productId={prod.productId}
        productTitle={prod.title}
      />
    </div>
  );
}

/* ───────── STATIC PAGES ───────── */
const Home=()=> (
  <div className="page home-page" style={{
    backgroundImage: 'url(/background.webp)',
    backgroundSize: 'cover',
    backgroundPosition: 'center',
    backgroundRepeat: 'no-repeat',
    minHeight: '100vh',
    position: 'relative'
  }}>
    {/* Overlay for better text readability */}
    <div style={{
      position: 'absolute',
      top: 0,
      left: 0,
      right: 0,
      bottom: 0,
      background: 'rgba(0, 0, 0, 0.4)',
      zIndex: 1
    }} />
    
    {/* Content */}
    <div style={{ position: 'relative', zIndex: 2, paddingTop: '15vh' }}>
      <h1 className="hero" style={{ color: 'white', textShadow: '2px 2px 4px rgba(0,0,0,0.8)' }}>
        <span className="accent">{T('voice_platform')}</span> {T('platform_for')} <br/>
        <span className="accent">{T('business_goods')}</span>
      </h1>
    </div>
  </div>
);
const Dashboard=()=> {
  const [recommendations, setRecommendations] = useState([]);
  const [loading, setLoading] = useState(true);
  const [user, setUser] = useState(null);
  const { lang } = useLang();

  useEffect(() => {
    const fetchData = async () => {
      setLoading(true);
      try {
        // Fetch current user
        const userData = await api.getCurrentUser();
        setUser(userData);

        // For now, use the same API call as search to get product recommendations
        // In a real app, this would be a dedicated recommendations endpoint
        const result = await api.search({ limit: 2, cursor: null, lang });
        // Duplicate the single product to simulate multiple recommendations
        const singleProduct = result.products[0];
        if (singleProduct) {
          const mockRecommendations = Array.from({ length: 2 }, (_, i) => ({
            ...singleProduct,
            productId: `${singleProduct.productId}-rec-${i}`,
            title: `${singleProduct.title} ${i + 1}`,
            price: Math.floor(Math.random() * 50000) + 5000, // Random price between 5k-55k
          }));
          setRecommendations(mockRecommendations);
        }
      } catch (error) {
        console.error('Failed to fetch data:', error);
      } finally {
        setLoading(false);
      }
    };

    fetchData();
  }, [lang]);

  const navigate = useNavigate();

  return (
    <div className="page" style={{ flexDirection: 'column', alignItems: 'stretch', justifyContent: 'flex-start', padding: '2rem 1rem' }}>
      <h1 className="hero" style={{ textAlign: 'center', marginBottom: '2rem' }}>
        Welcome back, {user?.username}
      </h1>
      
      <div style={{ maxWidth: '1200px', margin: '0 auto', width: '100%' }}>
        
        {/* Quick Actions */}
        <div style={{ marginBottom: '3rem' }}>
          <h2 style={{ fontSize: '1.5rem', fontWeight: '600', marginBottom: '1rem' }}>Quick Actions</h2>
          <div style={{ display: 'flex', gap: '1rem', flexWrap: 'wrap', justifyContent: 'center' }}>
            <button 
              className="pill outline action"
              onClick={() => navigate('/seller/products/create')}
              style={{ 
                color: 'var(--teal)', 
                borderColor: 'var(--teal)',
                fontWeight: '500'
              }}
            >
              + Add Product
            </button>
            <button 
              className="pill outline action"
              onClick={() => navigate('/chat')}
              style={{ 
                color: 'var(--teal)', 
                borderColor: 'var(--teal)',
                fontWeight: '500'
              }}
            >
              View Messages
            </button>
            <button 
              className="pill outline action"
              onClick={() => navigate('/settings')}
              style={{ 
                color: 'var(--teal)', 
                borderColor: 'var(--teal)',
                fontWeight: '500'
              }}
            >
              Account Settings
            </button>
          </div>
        </div>

        {/* Product Recommendations */}
        <div>
          <h2 style={{ fontSize: '1.5rem', fontWeight: '600', marginBottom: '1rem' }}>Recommended for You</h2>
          
          {loading ? (
            <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem', width: '100%', maxWidth: '44rem', margin: '0 auto' }}>
              {Array.from({ length: 2 }).map((_, i) => (
                <div key={i} className="product chat-product skeleton">
                  <div className="chat-prod-img skeleton-bg" />
                  <div className="chat-prod-info">
                    <div className="chat-prod-title skeleton-bg" style={{ width: '60%', height: 24, marginBottom: 8 }} />
                    <div className="chat-prod-meta">
                      <span className="chat-prod-price skeleton-bg" style={{ width: 80, height: 18 }} />
                      <span className="chat-prod-cond skeleton-bg" style={{ width: 120, height: 18 }} />
                    </div>
                    <div className="chat-prod-desc skeleton-bg" style={{ width: '100%', height: 32, margin: '8px 0' }} />
                    <div className="chat-prod-actions">
                      <span className="pill outline action skeleton-bg" style={{ width: 120, height: 32 }} />
                      <span className="pill outline action skeleton-bg" style={{ width: 100, height: 32, marginLeft: 8 }} />
                    </div>
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem', width: '100%', maxWidth: '44rem', margin: '0 auto' }}>
              {recommendations.map((product) => (
                <div 
                  key={product.productId} 
                  className="product chat-product" 
                  onClick={() => navigate(`/product/${product.productId}`)}
                  style={{ cursor: 'pointer' }}
                >
                  <div className="chat-prod-img" style={{ backgroundImage: `url(${product.thumbnail})` }} />
                  <div className="chat-prod-info">
                    <div className="chat-prod-title">{product.title}</div>
                    <div className="chat-prod-meta">
                      <span className="chat-prod-price">₹{product.price}</span>
                      <span className="chat-prod-cond">{product.conditionDesc}</span>
                    </div>
                    <div className="chat-prod-desc">{product.description}</div>
                    <div className="chat-prod-actions">
                      <button 
                        className="pill outline action" 
                        onClick={(e) => { 
                          e.stopPropagation(); 
                          alert(`${T('bought_for')} ₹${product.price}`);
                        }}
                      >
                        {T('buy_now')} ₹{product.price}
                      </button>
                      <button 
                        className="pill outline action" 
                        onClick={(e) => { 
                          e.stopPropagation(); 
                          navigate(`/product/${product.productId}`, { state: { openInquiry: true } });
                        }}
                      >
                        {T('inquire')}
                      </button>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
          
          {!loading && recommendations.length === 0 && (
            <div style={{ textAlign: 'center', padding: '2rem', color: 'var(--dark-grey)' }}>
              <p>No recommendations available at the moment.</p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

const Sell = () => {
  const [formData, setFormData] = useState({
    title: '',
    description: '',
    product_type: 'new',
    category: 'Other',
    tags: ''
  });
  const [thumbnailFile, setThumbnailFile] = useState(null);
  const [galleryFiles, setGalleryFiles] = useState([]);
  const [loading, setLoading] = useState(false);
  const [message, setMessage] = useState({ text: '', type: '' });
  
  const navigate = useNavigate();

  // Category options that match the ProductCategory enum
  const categoryOptions = [
    'Smartphones', 'Computers', 'Audio', 'Cameras', 'Gaming', 'Wearables', 'HomeElectronics',
    'MensClothing', 'WomensClothing', 'Shoes', 'Accessories', 'Jewelry', 'Bags', 'Beauty',
    'Furniture', 'HomeDecor', 'Kitchen', 'Garden', 'HomeTools', 'HomeImprovement',
    'FitnessEquipment', 'OutdoorGear', 'SportsEquipment', 'Bicycles', 'WaterSports', 'WinterSports',
    'CarParts', 'Motorcycles', 'AutoTools', 'CarAccessories', 'Books', 'Music', 'Movies', 'VideoGames',
    'HealthEquipment', 'PersonalCare', 'Supplements', 'MedicalDevices', 'BabyClothing', 'Toys',
    'BabyGear', 'KidsElectronics', 'Collectibles', 'Antiques', 'Art', 'Crafts', 'OfficeSupplies',
    'IndustrialEquipment', 'BusinessEquipment', 'Other'
  ];

  const handleInputChange = (e) => {
    const { name, value } = e.target;
    setFormData(prev => ({
      ...prev,
      [name]: value
    }));
  };

  const handleThumbnailChange = (e) => {
    const file = e.target.files[0];
    if (file) {
      if (file.size > 5 * 1024 * 1024) { // 5MB limit
        setMessage({ text: T('file_size_limit'), type: 'error' });
        return;
      }
      setThumbnailFile(file);
    }
  };

  const handleGalleryChange = (e) => {
    const files = Array.from(e.target.files);
    if (files.length > 5) {
      setMessage({ text: T('max_gallery_images'), type: 'error' });
      return;
    }
    
    const validFiles = [];
    for (const file of files) {
      if (file.size > 5 * 1024 * 1024) { // 5MB limit per file
        setMessage({ text: T('gallery_file_size_limit'), type: 'error' });
        return;
      }
      validFiles.push(file);
    }
    
    setGalleryFiles(validFiles);
  };

  const handleSubmit = async (e) => {
    e.preventDefault();
    setLoading(true);
    setMessage({ text: '', type: '' });

    try {
      // Validate required fields
      if (!formData.title || !formData.description || !formData.category || !thumbnailFile) {
        setMessage({ text: T('fill_required_fields'), type: 'error' });
        setLoading(false);
        return;
      }

      // Create FormData object
      const submitFormData = new FormData();
      
      // Add thumbnail
      submitFormData.append('thumbnail', thumbnailFile);
      
      // Add gallery images
      galleryFiles.forEach((file) => {
        submitFormData.append('gallery', file);
      });
      
      // Add product data as JSON (the API expects field name "product", not "body")
      const productData = {
        title: formData.title,
        description: formData.description,
        product_type: formData.product_type, // Keep as lowercase - "new" or "used" 
        category: formData.category, // Category should already be in PascalCase from the select options
        tags: formData.tags ? formData.tags.split(',').map(tag => tag.trim()).filter(tag => tag) : []
      };
      
      submitFormData.append('product', JSON.stringify(productData));

      // Submit to API
      const result = await api.create(submitFormData);
      
      setMessage({ text: T('product_created_successfully'), type: 'success' });
      
      // Reset form
      setFormData({
        title: '',
        description: '',
        product_type: 'new',
        category: 'Other',
        tags: ''
      });
      setThumbnailFile(null);
      setGalleryFiles([]);
      
      // Redirect to product page or dashboard after a delay
      setTimeout(() => {
        if (result && result.product && result.product.product_id) {
          navigate(`/product/${result.product.product_id}`);
        } else if (result && result.product_id) {
          navigate(`/product/${result.product_id}`);
        } else {
          navigate('/dashboard');
        }
      }, 2000);
      
    } catch (error) {
      setMessage({ text: error.message || T('failed_to_create_product'), type: 'error' });
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="page">
      <div style={{ maxWidth: '800px', margin: '0 auto', padding: '2rem 1rem' }}>
        <h1 className="hero">{T('sell_your_product')}</h1>
        
        {message.text && (
          <div 
            style={{ 
              padding: '1rem', 
              marginBottom: '2rem', 
              borderRadius: '8px',
              backgroundColor: message.type === 'error' ? '#fee' : '#efe',
              color: message.type === 'error' ? '#c33' : '#363',
              border: `1px solid ${message.type === 'error' ? '#fcc' : '#cfc'}`
            }}
          >
            {message.text}
          </div>
        )}

        <form onSubmit={handleSubmit} style={{ display: 'flex', flexDirection: 'column', gap: '1.5rem' }}>
          
          {/* Basic Information */}
          <div>
            <label style={{ display: 'block', marginBottom: '0.5rem', fontWeight: '600' }}>
              {T('product_title')} *
            </label>
            <input
              type="text"
              name="title"
              value={formData.title}
              onChange={handleInputChange}
              placeholder={T('enter_product_title')}
              required
              style={{
                width: '100%',
                padding: '0.75rem',
                border: '1px solid var(--light-grey)',
                borderRadius: '8px',
                fontSize: '1rem'
              }}
            />
          </div>

          <div>
            <label style={{ display: 'block', marginBottom: '0.5rem', fontWeight: '600' }}>
              {T('description')} *
            </label>
            <textarea
              name="description"
              value={formData.description}
              onChange={handleInputChange}
              placeholder={T('describe_your_product')}
              required
              rows={4}
              style={{
                width: '100%',
                padding: '0.75rem',
                border: '1px solid var(--light-grey)',
                borderRadius: '8px',
                fontSize: '1rem',
                resize: 'vertical'
              }}
            />
          </div>

          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '1rem' }}>
            <div>
              <label style={{ display: 'block', marginBottom: '0.5rem', fontWeight: '600' }}>
                {T('category')} *
              </label>
              <select
                name="category"
                value={formData.category}
                onChange={handleInputChange}
                required
                style={{
                  width: '100%',
                  padding: '0.75rem',
                  border: '1px solid var(--light-grey)',
                  borderRadius: '8px',
                  fontSize: '1rem'
                }}
              >
                {categoryOptions.map(category => (
                  <option key={category} value={category}>
                    {category.replace(/([A-Z])/g, ' $1').trim()}
                  </option>
                ))}
              </select>
            </div>

            <div>
              <label style={{ display: 'block', marginBottom: '0.5rem', fontWeight: '600' }}>
                Product Type
              </label>
              <select
                name="product_type"
                value={formData.product_type}
                onChange={handleInputChange}
                style={{
                  width: '100%',
                  padding: '0.75rem',
                  border: '1px solid var(--light-grey)',
                  borderRadius: '8px',
                  fontSize: '1rem'
                }}
              >
                <option value="new">{T('new')}</option>
                <option value="used">Used</option>
              </select>
            </div>
          </div>

          <div>
            <label style={{ display: 'block', marginBottom: '0.5rem', fontWeight: '600' }}>
              Tags (comma-separated)
            </label>
            <input
              type="text"
              name="tags"
              value={formData.tags}
              onChange={handleInputChange}
              placeholder="e.g., apple, iphone, smartphone, unlocked"
              style={{
                width: '100%',
                padding: '0.75rem',
                border: '1px solid var(--light-grey)',
                borderRadius: '8px',
                fontSize: '1rem'
              }}
            />
          </div>

          {/* Image Upload */}
          <div>
            <label style={{ display: 'block', marginBottom: '0.5rem', fontWeight: '600' }}>
              {T('thumbnail_image')} *
            </label>
            <input
              type="file"
              accept="image/*"
              onChange={handleThumbnailChange}
              required
              style={{
                width: '100%',
                padding: '0.75rem',
                border: '1px solid var(--light-grey)',
                borderRadius: '8px',
                fontSize: '1rem'
              }}
            />
            {thumbnailFile && (
              <div style={{ marginTop: '0.5rem', fontSize: '0.9rem', color: 'var(--dark-grey)' }}>
                Selected: {thumbnailFile.name} ({(thumbnailFile.size / 1024 / 1024).toFixed(2)} MB)
              </div>
            )}
          </div>

          <div>
            <label style={{ display: 'block', marginBottom: '0.5rem', fontWeight: '600' }}>
              {T('gallery_images_optional')}
            </label>
            <input
              type="file"
              accept="image/*"
              multiple
              onChange={handleGalleryChange}
              style={{
                width: '100%',
                padding: '0.75rem',
                border: '1px solid var(--light-grey)',
                borderRadius: '8px',
                fontSize: '1rem'
              }}
            />
            {galleryFiles.length > 0 && (
              <div style={{ marginTop: '0.5rem', fontSize: '0.9rem', color: 'var(--dark-grey)' }}>
                Selected {galleryFiles.length} image(s): {galleryFiles.map(f => f.name).join(', ')}
              </div>
            )}
          </div>

          {/* Submit Button */}
          <div style={{ display: 'flex', gap: '1rem', marginTop: '1rem' }}>
            <button
              type="button"
              onClick={() => navigate('/dashboard')}
              style={{
                padding: '0.75rem 1.5rem',
                border: '1px solid var(--light-grey)',
                borderRadius: '8px',
                backgroundColor: 'white',
                color: 'var(--dark-grey)',
                fontSize: '1rem',
                cursor: 'pointer'
              }}
            >
              {T('cancel')}
            </button>
            <button
              type="submit"
              disabled={loading}
              style={{
                padding: '0.75rem 1.5rem',
                border: 'none',
                borderRadius: '8px',
                backgroundColor: loading ? 'var(--light-grey)' : 'var(--teal)',
                color: 'white',
                fontSize: '1rem',
                cursor: loading ? 'not-allowed' : 'pointer',
                flex: 1
              }}
            >
              {loading ? T('creating_product') : T('create_product')}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};

const Settings=()=> {
  const navigate = useNavigate();
  const [user, setUser] = useState(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const fetchUser = async () => {
      try {
        const userData = await api.getCurrentUser();
        setUser(userData);
      } catch (error) {
        console.error('Failed to fetch user data:', error);
      } finally {
        setLoading(false);
      }
    };

    fetchUser();
  }, []);
  
  const handleLogout = async () => {
    try {
      const success = await api.logout();
      if (success) {
        window.dispatchEvent(new StorageEvent('storage', { key: 'logged', newValue: 'false' }));
        navigate('/');
      } else {
        alert('Failed to logout. Please try again.');
      }
    } catch (error) {
      console.error('Logout error:', error);
      alert('Failed to logout. Please try again.');
    }
  };
  
  const handleDeleteAccount = async () => {
    if (window.confirm(T('delete_account_confirm'))) {
      // Since there's no delete account endpoint, we'll just log out and clear local data
      try {
        await api.logout();
        localStorage.clear();
        alert(T('account_deleted'));
        navigate('/');
      } catch (error) {
        console.error('Delete account error:', error);
        // Even if logout fails, clear local data
        localStorage.clear();
        alert(T('account_deleted'));
        navigate('/');
      }
    }
  };

  if (loading) {
    return (
      <div className="page" style={{ flexDirection: 'column', alignItems: 'center', justifyContent: 'center', padding: '2rem 1rem' }}>
        <div>Loading...</div>
      </div>
    );
  }
  
  return (
    <div className="page" style={{ flexDirection: 'column', alignItems: 'stretch', justifyContent: 'flex-start', padding: '2rem 1rem' }}>
      <h1 className="hero" style={{ textAlign: 'center', marginBottom: '2rem' }}>{T('settings')}</h1>
      
      <div className="settings-container" style={{ maxWidth: '600px', margin: '0 auto', width: '100%' }}>
        
        {/* Account Section */}
        <div className="settings-section" style={{ marginBottom: '2rem', padding: '1.5rem', border: '1px solid var(--grey)', borderRadius: '1rem' }}>
          <h2 style={{ marginBottom: '1rem', fontSize: '1.25rem', fontWeight: '600' }}>{T('account')}</h2>
          
          <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '1rem', background: 'var(--light-grey)', borderRadius: '0.5rem' }}>
              <div style={{ textAlign: 'left', marginRight: '2rem' }}>
                <div style={{ fontWeight: '500' }}>{T('username')}</div>
                <div style={{ color: 'var(--dark-grey)', fontSize: '0.9rem' }}>{user?.username || 'Loading...'}</div>
              </div>
              <button className="pill outline" style={{ fontSize: '0.9rem', color: 'var(--teal)', borderColor: 'var(--teal)' }}>{T('change')}</button>
            </div>
            
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '1rem', background: 'var(--light-grey)', borderRadius: '0.5rem' }}>
              <div style={{ textAlign: 'left', marginRight: '2rem' }}>
                <div style={{ fontWeight: '500' }}>{T('email')}</div>
                <div style={{ color: 'var(--dark-grey)', fontSize: '0.9rem' }}>{user?.email || 'Loading...'}</div>
              </div>
              <button className="pill outline" style={{ fontSize: '0.9rem', color: 'var(--teal)', borderColor: 'var(--teal)' }}>{T('change')}</button>
            </div>
            
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '1rem', background: 'var(--light-grey)', borderRadius: '0.5rem' }}>
              <div style={{ textAlign: 'left', marginRight: '2rem' }}>
                <div style={{ fontWeight: '500' }}>{T('password')}</div>
                <div style={{ color: 'var(--dark-grey)', fontSize: '0.9rem' }}>••••••••</div>
              </div>
              <button className="pill outline" style={{ fontSize: '0.9rem', color: 'var(--teal)', borderColor: 'var(--teal)' }}>{T('change')}</button>
            </div>
          </div>
        </div>
        
        {/* Actions */}
        <div className="settings-section" style={{ padding: '1.5rem', border: '1px solid #ef4444', borderRadius: '1rem' }}>          
          <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
            <button 
              className="pill outline" 
              onClick={handleLogout}
              style={{ 
                color: '#ef4444', 
                borderColor: '#ef4444',
                width: 'fit-content',
                fontWeight: '500'
              }}
            >
              {T('log_out')}
            </button>
            
            <button 
              className="pill outline" 
              onClick={handleDeleteAccount}
              style={{ 
                color: '#dc2626', 
                borderColor: '#dc2626',
                width: 'fit-content',
                fontWeight: '500'
              }}
            >
              {T('delete_account')}
            </button>
          </div>
        </div>
        
      </div>
    </div>
  );
};

/* ───────── ROOT ROUTER ───────── */
export default function App(){
  const [searchOverlay, setSearchOverlay] = useState({ isOpen: false, voiceMode: false, imageMode: false });
  const [inquiryOverlay, setInquiryOverlay] = useState({ isOpen: false, productId: null, productTitle: null });
  const [imageSearchOverlay, setImageSearchOverlay] = useState(false);
  const [productResultsOverlay, setProductResultsOverlay] = useState({ isOpen: false, products: [] });
  const [previousSearchState, setPreviousSearchState] = useState(null);
  const location = useLocation();
  const navigate = useNavigate();

  // Handle /search route to open overlay
  useEffect(() => {
    if (location.pathname === '/search') {
      const voiceMode = location.state?.voiceMode === true;
      const imageMode = location.state?.imageMode === true;
      
      if (imageMode) {
        setImageSearchOverlay(true);
      } else {
        setSearchOverlay({ isOpen: true, voiceMode, imageMode: false });
      }
      
      // Replace the URL to remove /search from the path
      navigate('/', { replace: true });
    }
  }, [location.pathname, location.state, navigate]);

  // Handle navigation back from product pages
  useEffect(() => {
    // If we have saved state and we're no longer on a product page, restore the overlay
    if (previousSearchState && !location.pathname.startsWith('/product/')) {
      // Use a small delay to ensure navigation is complete
      const timer = setTimeout(() => {
        setSearchOverlay(previousSearchState);
        setPreviousSearchState(null);
      }, 100);
      return () => clearTimeout(timer);
    }
  }, [location.pathname, previousSearchState]);

  const handleProductNavigation = (overlayState) => {
    setPreviousSearchState(overlayState);
  };

  const closeSearchOverlay = () => {
    setSearchOverlay({ isOpen: false, voiceMode: false, imageMode: false });
    setPreviousSearchState(null); // Clear saved state when manually closing
  };

  const closeInquiryOverlay = () => {
    setInquiryOverlay({ isOpen: false, productId: null, productTitle: null });
  };

  const handleImageSearchOpen = () => {
    setImageSearchOverlay(true);
  };

  const handleImageSearchClose = () => {
    setImageSearchOverlay(false);
  };

  const handleProductsFound = (products) => {
    setProductResultsOverlay({ isOpen: true, products });
  };

  const handleProductResultsClose = () => {
    setProductResultsOverlay({ isOpen: false, products: [] });
  };

  return(
    <>
      <Navbar 
        onSearchOpen={(voiceMode) => setSearchOverlay({ isOpen: true, voiceMode })}
        onImageSearchOpen={handleImageSearchOpen}
      />
      <Routes>
        <Route path="/"                    element={<Home/>}/>
        <Route path="/login"               element={<Auth mode="login"/>}/>
        <Route path="/signup"              element={<Auth mode="signup"/>}/>
        <Route path="/dashboard"           element={<Dashboard/>}/>
        <Route path="/settings"            element={<Settings/>}/>
        <Route path="/seller/products/create" element={<Sell/>}/>
        <Route path="/product/:id"         element={<ProductPage/>}/>
        <Route path="/chat"                element={<Chat/>}/>
        <Route path="/sell"                element={<Sell/>}/>
      </Routes>
      <SearchOverlay 
        isOpen={searchOverlay.isOpen}
        voiceMode={searchOverlay.voiceMode}
        onClose={closeSearchOverlay}
        onProductNavigation={handleProductNavigation}
      />
      <InquiryOverlay 
        isOpen={inquiryOverlay.isOpen}
        onClose={closeInquiryOverlay}
        productId={inquiryOverlay.productId}
        productTitle={inquiryOverlay.productTitle}
      />
      <ImageSearchOverlay
        isOpen={imageSearchOverlay}
        onClose={handleImageSearchClose}
        onProductsFound={handleProductsFound}
      />
      <ProductResultsOverlay
        isOpen={productResultsOverlay.isOpen}
        products={productResultsOverlay.products}
        onClose={handleProductResultsClose}
      />
    </>
  );
}