import { useEffect, useRef, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { T, useLang, LANGS } from './i18n';
import './App.css';

const STORAGE_CHATS = 'chat-conversations';
const STORAGE_ACTIVE = 'active-chat-id';

// Mock chat data
const getMockChats = (lang) => {
  const isHindi = lang === LANGS.HI;
  
  return [
    {
      id: 'chat_1',
      contactName: isHindi ? 'राज पटेल - विक्रेता' : 'Raj Patel - Seller',
      productTitle: isHindi ? 'औद्योगिक विजेट मॉडल 1' : 'Industrial Widget Model 1',
      lastMessage: isHindi ? 'धन्यवाद आपकी रुचि के लिए!' : 'Thank you for your interest!',
      timestamp: Date.now() - 3600000, // 1 hour ago
      unread: 2,
      messages: [
        {
          id: 1,
          role: 'seller',
          text: isHindi ? 'नमस्ते! आपकी पूछताछ मिली। क्या आप इस उत्पाद के बारे में और जानना चाहते हैं?' : 'Hello! I received your inquiry. Would you like to know more about this product?',
          timestamp: Date.now() - 7200000
        },
        {
          id: 2,
          role: 'buyer',
          text: isHindi ? 'हाँ, मुझे डिलीवरी के समय के बारे में जानना है।' : 'Yes, I need to know about delivery time.',
          timestamp: Date.now() - 3900000
        },
        {
          id: 3,
          role: 'seller',
          text: isHindi ? 'डिलीवरी आमतौर पर 3-5 व्यावसायिक दिनों में होती है।' : 'Delivery usually takes 3-5 business days.',
          timestamp: Date.now() - 3600000
        }
      ]
    },
    {
      id: 'chat_2',
      contactName: isHindi ? 'प्रिया शर्मा - विक्रेता' : 'Priya Sharma - Seller',
      productTitle: isHindi ? 'औद्योगिक विजेट मॉडल 5' : 'Industrial Widget Model 5',
      lastMessage: isHindi ? 'आपका आर्डर तैयार है।' : 'Your order is ready.',
      timestamp: Date.now() - 86400000, // 1 day ago
      unread: 0,
      messages: [
        {
          id: 1,
          role: 'seller',
          text: isHindi ? 'आपकी पूछताछ के लिए धन्यवाद!' : 'Thank you for your inquiry!',
          timestamp: Date.now() - 172800000
        },
        {
          id: 2,
          role: 'buyer',
          text: isHindi ? 'क्या यह अभी भी उपलब्ध है?' : 'Is this still available?',
          timestamp: Date.now() - 90000000
        },
        {
          id: 3,
          role: 'seller',
          text: isHindi ? 'हाँ, आपका आर्डर तैयार है।' : 'Yes, your order is ready.',
          timestamp: Date.now() - 86400000
        }
      ]
    }
  ];
};

function ChatList({ chats, activeChat, onChatSelect, lang }) {
  const formatTime = (timestamp) => {
    const now = Date.now();
    const diff = now - timestamp;
    const minutes = Math.floor(diff / 60000);
    const hours = Math.floor(diff / 3600000);
    const days = Math.floor(diff / 86400000);
    
    if (minutes < 60) return `${minutes}m`;
    if (hours < 24) return `${hours}h`;
    return `${days}d`;
  };

  return (
    <div className="chat-list">
      <div className="chat-list-header">
        <h2>{'Messages'}</h2>
      </div>
      <div className="chat-list-items">
        {chats.map(chat => (
          <div 
            key={chat.id}
            className={`chat-list-item ${activeChat?.id === chat.id ? 'active' : ''}`}
            onClick={() => onChatSelect(chat)}
          >
            <div className="chat-avatar">
              <span className="material-symbols-outlined">person</span>
            </div>
            <div className="chat-info">
              <div className="chat-header">
                <span className="chat-name">{chat.contactName}</span>
                <span className="chat-time">{formatTime(chat.timestamp)}</span>
              </div>
              <div className="chat-preview">
                <span className="chat-product">{chat.productTitle}</span>
              </div>
              <div className="chat-last-message">
                {chat.lastMessage}
              </div>
            </div>
            {chat.unread > 0 && (
              <div className="chat-unread">{chat.unread}</div>
            )}
          </div>
        ))}
        {chats.length === 0 && (
          <div className="no-chats">
            <span className="material-symbols-outlined">chat_bubble_outline</span>
            <p>{'No messages yet'}</p>
            <p className="help-text">{'Start an inquiry to begin chatting with sellers'}</p>
          </div>
        )}
      </div>
    </div>
  );
}

function ChatWindow({ chat, onSendMessage, lang }) {
  const [message, setMessage] = useState('');
  const [isListening, setIsListening] = useState(false);
  const messagesEndRef = useRef(null);
  const recognitionRef = useRef(null);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [chat?.messages]);

  useEffect(() => {
    // Initialize speech recognition
    if ('webkitSpeechRecognition' in window || 'SpeechRecognition' in window) {
      const SpeechRecognition = window.SpeechRecognition || window.webkitSpeechRecognition;
      recognitionRef.current = new SpeechRecognition();
      
      const recognition = recognitionRef.current;
      recognition.continuous = false;
      recognition.interimResults = false;
      recognition.lang = lang === 'hi' ? 'hi-IN' : 'en-US';
      
      recognition.onstart = () => {
        setIsListening(true);
      };
      
      recognition.onresult = (event) => {
        const transcript = event.results[0][0].transcript;
        setMessage(prev => prev + (prev ? ' ' : '') + transcript);
      };
      
      recognition.onend = () => {
        setIsListening(false);
      };
      
      recognition.onerror = (event) => {
        console.error('Speech recognition error:', event.error);
        setIsListening(false);
      };
    }
    
    return () => {
      if (recognitionRef.current) {
        recognitionRef.current.abort();
      }
    };
  }, [lang]);

  const handleSend = () => {
    if (!message.trim()) return;
    onSendMessage(message.trim());
    setMessage('');
  };

  const handleKeyPress = (e) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  const handleVoiceClick = () => {
    if (!recognitionRef.current) {
      alert('Speech recognition is not supported in your browser');
      return;
    }

    if (isListening) {
      recognitionRef.current.stop();
    } else {
      recognitionRef.current.start();
    }
  };

  const formatMessageTime = (timestamp) => {
    return new Date(timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  };

  if (!chat) {
    return (
      <div className="chat-placeholder">
        <span className="material-symbols-outlined">chat_bubble_outline</span>
        <h3>{'Select a conversation'}</h3>
        <p>{'Choose a conversation from the list to start messaging'}</p>
      </div>
    );
  }

  return (
    <div className="chat-window">
      <div className="chat-window-header">
        <div className="chat-contact">
          <div className="chat-avatar">
            <span className="material-symbols-outlined">person</span>
          </div>
          <div className="chat-contact-info">
            <h3>{chat.contactName}</h3>
            <p>{chat.productTitle}</p>
          </div>
        </div>
        <div className="chat-actions">
          <button className="circle">
            <span className="material-symbols-outlined">call</span>
          </button>
          <button className="circle">
            <span className="material-symbols-outlined">more_vert</span>
          </button>
        </div>
      </div>
      
      <div className="chat-messages">
        {chat.messages.map(msg => (
          <div key={msg.id} className={`message ${msg.role === 'buyer' ? 'own' : 'other'}`}>
            <div className="message-content">
              {msg.text}
            </div>
            <div className="message-time">
              {formatMessageTime(msg.timestamp)}
            </div>
          </div>
        ))}
        <div ref={messagesEndRef} />
      </div>

      <div className="chat-input-bar">
        <div className="chat-input-container">
          <textarea
            value={message}
            onChange={(e) => setMessage(e.target.value)}
            onKeyPress={handleKeyPress}
            placeholder={'Type a message...'}
            rows={1}
            className="chat-input"
          />
          <button 
            className={`chat-voice-btn ${isListening ? 'listening' : ''}`}
            onClick={handleVoiceClick}
            title={isListening ? 'Stop listening' : 'Start voice input'}
          >
            <span className="material-symbols-outlined">
              {isListening ? 'stop' : 'mic'}
            </span>
          </button>
          <button 
            className="chat-send-btn"
            onClick={handleSend}
            disabled={!message.trim()}
          >
            <span className="material-symbols-outlined">send</span>
          </button>
        </div>
      </div>
    </div>
  );
}

export default function Chat() {
  const navigate = useNavigate();
  const { lang } = useLang();
  const [chats, setChats] = useState(() => {
    const saved = localStorage.getItem(STORAGE_CHATS);
    if (saved) {
      try {
        return JSON.parse(saved);
      } catch {
        return getMockChats(lang);
      }
    }
    return getMockChats(lang);
  });
  
  const [activeChat, setActiveChat] = useState(() => {
    const activeChatId = localStorage.getItem(STORAGE_ACTIVE);
    return chats.find(chat => chat.id === activeChatId) || null;
  });

  // Check for new inquiry messages on mount
  useEffect(() => {
    const inquiryData = sessionStorage.getItem('pending-inquiry-message');
    if (inquiryData) {
      try {
        const inquiry = JSON.parse(inquiryData);
        handleInquiryMessage(inquiry);
        sessionStorage.removeItem('pending-inquiry-message');
      } catch (error) {
        console.error('Error processing inquiry message:', error);
      }
    }
  }, []);

  useEffect(() => {
    localStorage.setItem(STORAGE_CHATS, JSON.stringify(chats));
  }, [chats]);

  useEffect(() => {
    if (activeChat) {
      localStorage.setItem(STORAGE_ACTIVE, activeChat.id);
    }
  }, [activeChat]);

  const handleInquiryMessage = (inquiry) => {
    const { productId, productTitle, questions, answers } = inquiry;
    
    // Format the inquiry message
    const isHindi = lang === LANGS.HI;
    const inquiryText = isHindi 
      ? `नई पूछताछ: ${productTitle}\n\nप्रश्न और उत्तर:\n${questions.map((q, i) => `${i + 1}. ${q.question}\n   उत्तर: ${answers[i] || 'N/A'}`).join('\n\n')}`
      : `New Inquiry: ${productTitle}\n\nQuestions & Answers:\n${questions.map((q, i) => `${i + 1}. ${q.question}\n   Answer: ${answers[i] || 'N/A'}`).join('\n\n')}`;
    
    // Create or find existing chat
    const chatId = `inquiry_${productId}_${Date.now()}`;
    const contactName = isHindi ? 'नई पूछताछ - विक्रेता' : 'New Inquiry - Seller';
    
    const newMessage = {
      id: Date.now(),
      role: 'buyer',
      text: inquiryText,
      timestamp: Date.now()
    };

    const existingChatIndex = chats.findIndex(chat => 
      chat.productTitle === productTitle
    );

    if (existingChatIndex !== -1) {
      // Add to existing chat
      const updatedChats = [...chats];
      updatedChats[existingChatIndex].messages.push(newMessage);
      updatedChats[existingChatIndex].lastMessage = isHindi ? 'नई पूछताछ भेजी गई' : 'New inquiry sent';
      updatedChats[existingChatIndex].timestamp = Date.now();
      setChats(updatedChats);
      setActiveChat(updatedChats[existingChatIndex]);
    } else {
      // Create new chat
      const newChat = {
        id: chatId,
        contactName,
        productTitle,
        lastMessage: isHindi ? 'नई पूछताछ भेजी गई' : 'New inquiry sent',
        timestamp: Date.now(),
        unread: 0,
        messages: [newMessage]
      };
      
      const updatedChats = [newChat, ...chats];
      setChats(updatedChats);
      setActiveChat(newChat);
    }
  };

  const handleChatSelect = (chat) => {
    setActiveChat(chat);
    // Mark as read
    if (chat.unread > 0) {
      const updatedChats = chats.map(c => 
        c.id === chat.id ? { ...c, unread: 0 } : c
      );
      setChats(updatedChats);
    }
  };

  const handleSendMessage = (messageText) => {
    if (!activeChat) return;

    const newMessage = {
      id: Date.now(),
      role: 'buyer',
      text: messageText,
      timestamp: Date.now()
    };

    const updatedChats = chats.map(chat => {
      if (chat.id === activeChat.id) {
        return {
          ...chat,
          messages: [...chat.messages, newMessage],
          lastMessage: messageText,
          timestamp: Date.now()
        };
      }
      return chat;
    });

    setChats(updatedChats);
    setActiveChat(prev => ({
      ...prev,
      messages: [...prev.messages, newMessage],
      lastMessage: messageText,
      timestamp: Date.now()
    }));

    // Simulate seller response (for demo)
    setTimeout(() => {
      const sellerResponse = {
        id: Date.now() + 1,
        role: 'seller',
        text: lang === LANGS.HI 
          ? 'आपके संदेश के लिए धन्यवाद! मैं जल्द ही उत्तर दूंगा।'
          : 'Thank you for your message! I will respond shortly.',
        timestamp: Date.now()
      };

      setChats(prev => prev.map(chat => {
        if (chat.id === activeChat.id) {
          return {
            ...chat,
            messages: [...chat.messages, sellerResponse],
            lastMessage: sellerResponse.text,
            timestamp: Date.now(),
            unread: 1
          };
        }
        return chat;
      }));

      setActiveChat(prev => ({
        ...prev,
        messages: [...prev.messages, sellerResponse],
        lastMessage: sellerResponse.text,
        timestamp: Date.now()
      }));
    }, 2000);
  };

  return (
    <div className="chat-page">
      <div className="chat-container">
        <ChatList 
          chats={chats}
          activeChat={activeChat}
          onChatSelect={handleChatSelect}
          lang={lang}
          navigate={navigate}
        />
        <ChatWindow 
          chat={activeChat}
          onSendMessage={handleSendMessage}
          lang={lang}
          navigate={navigate}
        />
      </div>
    </div>
  );
}
