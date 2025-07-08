import { createContext, useContext, useEffect, useState, useCallback } from 'react';

const LangCtx = createContext();

export const LANGS = { EN: 'en', HI: 'hi' };

// Translation cache to avoid repeated API calls
const translationCache = new Map();

// Google Translate API function with multiple fallback methods
async function translateText(text, targetLang = 'hi') {
  // Don't translate if target language is English or text is empty
  if (targetLang === 'en' || !text || text.trim() === '') {
    return text;
  }

  // Check cache first
  const cacheKey = `${text}_${targetLang}`;
  if (translationCache.has(cacheKey)) {
    return translationCache.get(cacheKey);
  }

  try {
    // Method 1: Try the original Google Translate endpoint
    const url = new URL('https://clients5.google.com/translate_a/t');
    url.searchParams.set('client', 'dict-chrome-ex');
    url.searchParams.set('sl', 'auto');
    url.searchParams.set('tl', targetLang);
    url.searchParams.set('q', text);

    console.log('Attempting translation for:', text);
    const response = await fetch(url, {
      method: 'GET',
      mode: 'cors',
      headers: {
        'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36'
      }
    });
    
    if (!response.ok) {
      throw new Error(`Translation failed: ${response.status}`);
    }

    const data = await response.json();
    console.log('Translation API response:', data);
    
    // Handle the actual API response format: [["translated_text","source_lang"]]
    let translation = text;
    if (Array.isArray(data) && data.length > 0 && Array.isArray(data[0]) && data[0].length > 0) {
      translation = data[0][0];
    }
    
    // Cache the translation
    translationCache.set(cacheKey, translation);
    console.log('Translation successful:', text, '->', translation);
    
    return translation;
  } catch (error) {
    console.warn('Primary translation method failed:', error);
    
    // Method 2: Fallback to alternative Google Translate endpoint
    try {
      const fallbackUrl = `https://translate.googleapis.com/translate_a/single?client=gtx&sl=auto&tl=${targetLang}&dt=t&q=${encodeURIComponent(text)}`;
      const fallbackResponse = await fetch(fallbackUrl);
      
      if (fallbackResponse.ok) {
        const fallbackData = await fallbackResponse.json();
        console.log('Fallback API response:', fallbackData);
        
        if (Array.isArray(fallbackData) && fallbackData[0] && Array.isArray(fallbackData[0]) && fallbackData[0][0]) {
          const translation = fallbackData[0][0][0];
          translationCache.set(cacheKey, translation);
          console.log('Fallback translation successful:', text, '->', translation);
          return translation;
        }
      }
    } catch (fallbackError) {
      console.warn('Fallback translation failed:', fallbackError);
    }
    
    // Method 3: Use a comprehensive hardcoded translation for common terms as last resort
    const commonTranslations = {
      // Navigation and UI
      'Home': 'होम',
      'Search': 'खोजें',
      'Back': 'वापस',
      'Chat': 'चैट',
      'Login': 'लॉगिन',
      'Logout': 'लॉग आउट',
      'Settings': 'सेटिंग्स',
      'Dashboard': 'डैशबोर्ड',
      'Sell': 'बेचें',
      'Products': 'उत्पाद',
      'Price': 'कीमत',
      'Description': 'विवरण',
      'Category': 'श्रेणी',
      'Inquire': 'पूछताछ',
      'English': 'English',
      'हिन्दी': 'हिन्दी',
      
      // Common actions
      'Submit': 'सबमिट करें',
      'Cancel': 'रद्द करें',
      'Save': 'सेव करें',
      'Delete': 'डिलीट करें',
      'Edit': 'संपादित करें',
      'View': 'देखें',
      'Close': 'बंद करें',
      'Open': 'खोलें',
      'Start': 'शुरू करें',
      'Stop': 'रोकें',
      'Continue': 'जारी रखें',
      'Next': 'अगला',
      'Previous': 'पिछला',
      'Yes': 'हाँ',
      'No': 'नहीं',
      
      // Product-related
      'Quality': 'गुणवत्ता',
      'Condition': 'स्थिति',
      'Brand': 'ब्रांड',
      'Model': 'मॉडल',
      'Quantity': 'मात्रा',
      'Available': 'उपलब्ध',
      'Sold': 'बेचा गया',
      'New': 'नया',
      'Used': 'प्रयुक्त',
      
      // Voice and search
      'Voice-First': 'वॉयस-फर्स्ट',
      'Platform for': 'के लिए प्लेटफॉर्म',
      'Business Goods': 'व्यापारिक सामान',
      'Type your message': 'अपना संदेश टाइप करें',
      'Send': 'भेजें',
      'Listening...': 'सुन रहा है...',
      'Start Voice Search': 'वॉयस सर्च शुरू करें',
      
      // Common phrases
      'Hello World': 'हैलो वर्ल्ड',
      'Test Translation': 'अनुवाद परीक्षण',
      'Loading...': 'लोड हो रहा है...',
      'Please wait': 'कृपया प्रतीक्षा करें',
      'Error': 'त्रुटि',
      'Success': 'सफलता',
      'Warning': 'चेतावनी',
      'Information': 'जानकारी'
    };
    
    if (commonTranslations[text]) {
      const translation = commonTranslations[text];
      translationCache.set(cacheKey, translation);
      console.log('Used hardcoded translation:', text, '->', translation);
      return translation;
    }
    
    console.warn('All translation methods failed for:', text);
    return text; // Return original text if all translation methods fail
  }
}

// Hook for getting current language
export function useLang() {
  return useContext(LangCtx);
}

// Translation function - now uses live translation
export function T(text) {
  const { lang, translations } = useContext(LangCtx);
  
  // If English or no translation available, return original text
  if (lang === LANGS.EN) {
    return text;
  }

  // If we have a cached translation, use it
  if (translations.has(text)) {
    return translations.get(text);
  }

  // Return original text while translation is loading
  return text;
}

// Enhanced T function for components that can handle async
export function useTranslation(text) {
  const { lang, translate } = useContext(LangCtx);
  const [translatedText, setTranslatedText] = useState(text);

  useEffect(() => {
    if (lang === LANGS.EN) {
      setTranslatedText(text);
      return;
    }

    // Trigger translation and update when complete
    translate(text).then(setTranslatedText).catch(() => {
      // If translation fails, keep original text
      setTranslatedText(text);
    });
  }, [text, lang]); // Removed translate dependency to prevent infinite loops

  return translatedText;
}

// Translation provider
export function LangProvider({ children }) {
  const [lang, setLang] = useState(
    localStorage.getItem('gp-lang') || LANGS.EN
  );
  const [translations, setTranslations] = useState(new Map());

  useEffect(() => {
    localStorage.setItem('gp-lang', lang);
  }, [lang]);

  // Function to translate text and update cache
  const translate = useCallback(async (text) => {
    if (lang === LANGS.EN || !text) {
      return text;
    }

    if (translations.has(text)) {
      return translations.get(text);
    }

    const translated = await translateText(text, lang);
    setTranslations(prev => new Map(prev).set(text, translated));
    return translated;
  }, [lang, translations]);

  // Toggle language
  const toggle = () => {
    setLang(l => l === LANGS.EN ? LANGS.HI : LANGS.EN);
  };

  // Auto-translate content when language changes to Hindi
  useEffect(() => {
    console.log('Language changed to:', lang);
    if (lang === LANGS.HI) {
      console.log('Hindi selected - manual translation mode enabled');
      // Note: Automatic translation disabled to prevent jittering
      // Translation will be handled manually via Test Translation button or component-level translation
    } else if (lang === LANGS.EN) {
      // When switching back to English, reload the page to restore original text
      console.log('Switching back to English - reloading page');
      window.location.reload();
    }
  }, [lang]);

  return (
    <LangCtx.Provider value={{ lang, toggle, translations, translate }}>
      {children}
    </LangCtx.Provider>
  );
}