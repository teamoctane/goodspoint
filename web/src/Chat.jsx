import { useEffect, useRef, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { T, useLang, LANGS } from './i18n';
import './App.css';

// API functions for chat
const chatAPI = {
  // Get all conversations
  getConversations: async () => {
    try {
      const response = await fetch('/chat/conversations', {
        credentials: 'include',
        headers: {
          'Content-Type': 'application/json'
        }
      });
      
      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }
      
      const data = await response.json();
      return data.conversations || [];
    } catch (error) {
      console.error('Error fetching conversations:', error);
      return [];
    }
  },

  // Get messages for a specific conversation
  getMessages: async (otherUserId) => {
    try {
      const response = await fetch(`/chat/${otherUserId}/messages`, {
        credentials: 'include',
        headers: {
          'Content-Type': 'application/json'
        }
      });
      
      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }
      
      const data = await response.json();
      return data.messages || [];
    } catch (error) {
      console.error('Error fetching messages:', error);
      return [];
    }
  },

  // Send a message using FormData (as expected by the backend)
  sendMessage: async (otherUserId, messageText) => {
    try {
      const formData = new FormData();
      formData.append('content', messageText);

      const response = await fetch(`/chat/${otherUserId}/messages`, {
        method: 'POST',
        credentials: 'include',
        body: formData
      });
      
      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }
      
      const data = await response.json();
      return data;
    } catch (error) {
      console.error('Error sending message:', error);
      throw error;
    }
  }
};

// Helper function to format timestamp
const formatTime = (timestamp) => {
  const date = new Date(timestamp * 1000); // Backend uses seconds since epoch
  const now = new Date();
  const diffMs = now - date;
  const diffMins = Math.floor(diffMs / 60000);
  const diffHours = Math.floor(diffMs / 3600000);
  const diffDays = Math.floor(diffMs / 86400000);

  if (diffMins < 1) return 'Just now';
  if (diffMins < 60) return `${diffMins}m ago`;
  if (diffHours < 24) return `${diffHours}h ago`;
  if (diffDays < 7) return `${diffDays}d ago`;
  
  return date.toLocaleDateString();
};

function ChatList({ conversations, activeConversation, onConversationSelect, lang }) {
  return (
    <div className="chat-list">
      <div className="chat-list-header">
        <h2>{T('messages')}</h2>
      </div>
      <div className="chat-list-items">
        {conversations.map(conv => (
          <div 
            key={conv.conversation_id}
            className={`chat-list-item ${activeConversation?.conversation_id === conv.conversation_id ? 'active' : ''}`}
            onClick={() => onConversationSelect(conv)}
          >
            <div className="chat-avatar">
              <span className="material-symbols-outlined">person</span>
            </div>
            <div className="chat-info">
              <div className="chat-header">
                <span className="chat-name">
                  {lang === LANGS.HI ? 'उपयोगकर्ता' : 'User'} {conv.other_participant_id}
                </span>
                <span className="chat-time">{formatTime(conv.last_message_at)}</span>
              </div>
              <div className="chat-preview">
                <span className="chat-product">
                  {lang === LANGS.HI ? 'बातचीत' : 'Conversation'}
                </span>
              </div>
            </div>
          </div>
        ))}
        {conversations.length === 0 && (
          <div className="no-chats">
            <span className="material-symbols-outlined">chat_bubble_outline</span>
            <p>{T('no_messages_yet')}</p>
            <p className="help-text">
              {T('start_inquiry_to_chat')}
            </p>
          </div>
        )}
      </div>
    </div>
  );
}

function ChatWindow({ conversation, messages, onSendMessage, lang, currentUserId, isLoading }) {
  const [message, setMessage] = useState('');
  const [isListening, setIsListening] = useState(false);
  const [sendingMessage, setSendingMessage] = useState(false);
  const messagesEndRef = useRef(null);
  const recognitionRef = useRef(null);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

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

  const handleSend = async () => {
    if (!message.trim() || sendingMessage || !conversation) return;
    
    setSendingMessage(true);
    try {
      await onSendMessage(conversation.other_participant_id, message.trim());
      setMessage('');
    } catch (error) {
      console.error('Failed to send message:', error);
    } finally {
      setSendingMessage(false);
    }
  };

  const handleKeyPress = (e) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  const handleVoiceClick = () => {
    if (!recognitionRef.current) {
      alert(lang === LANGS.HI ? 'आपके ब्राउज़र में वॉयस रिकॉग्निशन समर्थित नहीं है' : 'Speech recognition is not supported in your browser');
      return;
    }

    if (isListening) {
      recognitionRef.current.stop();
    } else {
      recognitionRef.current.start();
    }
  };

  const formatMessageTime = (timestamp) => {
    return new Date(timestamp * 1000).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  };

  if (!conversation) {
    return (
      <div className="chat-placeholder">
        <span className="material-symbols-outlined">chat_bubble_outline</span>
        <h3>{T('select_conversation')}</h3>
        <p>{T('choose_conversation_to_start')}</p>
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
            <h3>
              {lang === LANGS.HI ? 'उपयोगकर्ता' : 'User'} {conversation.other_participant_id}
            </h3>
            <p>{lang === LANGS.HI ? 'बातचीत' : 'Conversation'}</p>
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
        {isLoading && (
          <div className="message-loading">
            {T('loading_messages')}
          </div>
        )}
        {messages.map(msg => (
          <div key={msg.message_id} className={`message ${msg.sender_id === currentUserId ? 'own' : 'other'}`}>
            <div className="message-content">
              {msg.content}
            </div>
            <div className="message-time">
              {formatMessageTime(msg.created_at)}
              {msg.is_edited && <span className="edited-indicator"> ({T('edited')})</span>}
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
            placeholder={lang === LANGS.HI ? 'एक संदेश टाइप करें...' : 'Type a message...'}
            rows={1}
            className="chat-input"
            disabled={sendingMessage}
          />
          <button 
            className={`chat-voice-btn ${isListening ? 'listening' : ''}`}
            onClick={handleVoiceClick}
            title={isListening ? 
              (lang === LANGS.HI ? 'सुनना बंद करें' : 'Stop listening') : 
              (lang === LANGS.HI ? 'वॉयस इनपुट शुरू करें' : 'Start voice input')
            }
            disabled={sendingMessage}
          >
            <span className="material-symbols-outlined">
              {isListening ? 'stop' : 'mic'}
            </span>
          </button>
          <button 
            className="chat-send-btn"
            onClick={handleSend}
            disabled={!message.trim() || sendingMessage}
          >
            <span className="material-symbols-outlined">
              {sendingMessage ? 'hourglass_empty' : 'send'}
            </span>
          </button>
        </div>
      </div>
    </div>
  );
}

export default function Chat() {
  const navigate = useNavigate();
  const { lang } = useLang();
  const [conversations, setConversations] = useState([]);
  const [activeConversation, setActiveConversation] = useState(null);
  const [messages, setMessages] = useState([]);
  const [currentUserId, setCurrentUserId] = useState(null);
  const [loading, setLoading] = useState(true);
  const [messagesLoading, setMessagesLoading] = useState(false);

  // Load conversations on component mount
  useEffect(() => {
    loadConversations();
    loadCurrentUser();
  }, []);

  // Load messages when active conversation changes
  useEffect(() => {
    if (activeConversation) {
      loadMessages(activeConversation.other_participant_id);
    } else {
      setMessages([]);
    }
  }, [activeConversation]);

  // Check for auth status and redirect if needed
  useEffect(() => {
    const checkAuth = async () => {
      try {
        const response = await fetch('/auth/user', {
          credentials: 'include'
        });
        if (!response.ok) {
          navigate('/');
          return;
        }
        const userData = await response.json();
        setCurrentUserId(userData.user.uid);
      } catch (error) {
        console.error('Auth check failed:', error);
        navigate('/');
      }
    };
    checkAuth();
  }, [navigate]);

  const loadCurrentUser = async () => {
    try {
      const response = await fetch('/auth/user', {
        credentials: 'include'
      });
      if (response.ok) {
        const userData = await response.json();
        setCurrentUserId(userData.user.uid);
      }
    } catch (error) {
      console.error('Failed to get current user:', error);
    }
  };

  const loadConversations = async () => {
    setLoading(true);
    try {
      const convs = await chatAPI.getConversations();
      setConversations(convs);
    } catch (error) {
      console.error('Failed to load conversations:', error);
    } finally {
      setLoading(false);
    }
  };

  const loadMessages = async (otherUserId) => {
    setMessagesLoading(true);
    try {
      const msgs = await chatAPI.getMessages(otherUserId);
      setMessages(msgs);
    } catch (error) {
      console.error('Failed to load messages:', error);
      setMessages([]);
    } finally {
      setMessagesLoading(false);
    }
  };

  const handleConversationSelect = (conversation) => {
    setActiveConversation(conversation);
  };

  const handleSendMessage = async (otherUserId, messageText) => {
    try {
      const response = await chatAPI.sendMessage(otherUserId, messageText);
      
      if (response.status === 'ok') {
        // Add the new message to the current messages
        const newMessage = response.message;
        setMessages(prev => [...prev, newMessage]);
        
        // Update conversations list to reflect the new message
        setConversations(prev => prev.map(conv => 
          conv.other_participant_id === otherUserId 
            ? { ...conv, last_message_at: newMessage.created_at }
            : conv
        ));
      }
    } catch (error) {
      console.error('Failed to send message:', error);
      throw error;
    }
  };

  // Handle inquiry messages from other parts of the app
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

  const handleInquiryMessage = async (inquiry) => {
    const { productTitle, questions, answers, sellerId } = inquiry;
    
    if (!sellerId) {
      console.error('No seller ID provided in inquiry');
      return;
    }

    // Format the inquiry message
    const isHindi = lang === LANGS.HI;
    const inquiryText = isHindi 
      ? `नई पूछताछ: ${productTitle}\n\nप्रश्न और उत्तर:\n${questions.map((q, i) => `${i + 1}. ${q.question}\n   उत्तर: ${answers[i] || 'N/A'}`).join('\n\n')}`
      : `New Inquiry: ${productTitle}\n\nQuestions & Answers:\n${questions.map((q, i) => `${i + 1}. ${q.question}\n   Answer: ${answers[i] || 'N/A'}`).join('\n\n')}`;
    
    try {
      // Send the inquiry message
      await handleSendMessage(sellerId, inquiryText);
      
      // Reload conversations to ensure the new conversation appears
      await loadConversations();
      
      // Find and select the conversation with this seller
      const conversation = conversations.find(conv => conv.other_participant_id === sellerId);
      if (conversation) {
        setActiveConversation(conversation);
      }
    } catch (error) {
      console.error('Failed to send inquiry message:', error);
    }
  };

  if (loading) {
    return (
      <div className="chat-page">
        <div className="chat-loading">
          {T('loading')}
        </div>
      </div>
    );
  }

  return (
    <div className="chat-page">
      <div className="chat-container">
        <ChatList 
          conversations={conversations}
          activeConversation={activeConversation}
          onConversationSelect={handleConversationSelect}
          lang={lang}
        />
        <ChatWindow 
          conversation={activeConversation}
          messages={messages}
          onSendMessage={handleSendMessage}
          lang={lang}
          currentUserId={currentUserId}
          isLoading={messagesLoading}
        />
      </div>
    </div>
  );
}
