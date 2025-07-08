import { useEffect, useRef, useState } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { TextInputBar, VoiceInputBar } from './SearchChatInput';
import './App.css';
import { T } from './i18n';

const STORAGE = 'inquiry-chat';

// Mock questions API
const getQuestions = async (productId) => {
  // Simulate API call
  return {
    status: "ok",
    questions: {
      questions: [
        {
          id: "q_1",
          question: "Is the product capable of performing tasks independently?",
          question_type: "YesNo"
        },
        {
          id: "q_2",
          question: "What is the primary function you need from this product?",
          question_type: "FreeResponse"
        },
        {
          id: "q_3",
          question: "Does the product require any external maintenance or support?",
          question_type: "YesNo"
        },
        {
          id: "q_4",
          question: "Can the product be used in a team or collaborative setting?",
          question_type: "YesNo"
        },
        {
          id: "q_5",
          question: "What are the specific capabilities you're looking for?",
          question_type: "FreeResponse"
        }
      ]
    }
  };
};

export default function InquiryChat({ voiceMode: propVoiceMode, onClose, productId, productTitle }) {
  const navigate = useNavigate();
  const location = useLocation();
  const scrollRef = useRef(null);
  const inputRef = useRef(null);
  const bottomRef = useRef(null);
  const inputStartRef = useRef();
  const voiceInputRef = useRef();
  
  // Get voice mode from navigation state or prop
  const voiceMode = propVoiceMode || location.state?.voiceMode === true;
  const [draft, setDraft] = useState('');

  const [messages, setMsgs] = useState(() => {
    const saved = sessionStorage.getItem(STORAGE);
    if (saved) {
      try {
        return JSON.parse(saved);
      } catch {
        return [];
      }
    }
    return []; // Start with empty messages, questions will be added after loading
  });
  
  const msgsRef = useRef(messages);
  const [questions, setQuestions] = useState([]);
  const [currentQuestionIndex, setCurrentQuestionIndex] = useState(0);
  const [isComplete, setIsComplete] = useState(false);
  const [aiMessageAdded, setAiMessageAdded] = useState(false);

  useEffect(() => { msgsRef.current = messages; }, [messages]);

  // Load questions on mount
  useEffect(() => {
    getQuestions(productId).then(response => {
      if (response.status === 'ok') {
        setQuestions(response.questions.questions);
        // Ask first question immediately without intro
        if (response.questions.questions.length > 0) {
          const firstQ = response.questions.questions[0];
          setMsgs([{
            role: 'ai',
            text: firstQ.question + (firstQ.question_type === 'YesNo' ? ' (Yes/No)' : '')
          }]);
          setAiMessageAdded(true);
        }
      }
    });
  }, [productId]);

  useEffect(() => {
    sessionStorage.setItem(STORAGE, JSON.stringify(messages));
  }, [messages]);

  // Auto-scroll when AI message is added
  useEffect(() => {
    if (aiMessageAdded && bottomRef.current) {
      console.log('AI message state triggered, scrolling to bottom');
      setTimeout(() => {
        if (bottomRef.current) {
          console.log('Executing AI message scroll');
          bottomRef.current.scrollIntoView({ behavior: 'smooth', block: 'end' });
        }
      }, 100);
      setAiMessageAdded(false);
    }
  }, [aiMessageAdded]);

  // Auto-scroll when new messages are added
  useEffect(() => {
    if (bottomRef.current) {
      setTimeout(() => {
        bottomRef.current.scrollIntoView({ behavior: 'smooth', block: 'end' });
      }, 100);
    }
  }, [messages.length]);

  // Clean up on unmount
  useEffect(() => {
    return () => {
      if (voiceMode && voiceInputRef.current) {
        console.log('Stopping voice recognition on unmount');
        if (voiceInputRef.current.stop) {
          voiceInputRef.current.stop();
        }
      }
      sessionStorage.removeItem(STORAGE);
    };
  }, [voiceMode]);

  const handleSend = async (text) => {
    if (!text.trim()) return;
    
    // Validate Yes/No questions
    if (currentQuestionIndex < questions.length) {
      const currentQ = questions[currentQuestionIndex];
      if (currentQ.question_type === 'YesNo') {
        const normalized = text.toLowerCase().trim();
        const yesAnswers = ['yes', 'y', 'yeah', 'yep', 'true', 'correct', 'right', 'affirmative'];
        const noAnswers = ['no', 'n', 'nope', 'false', 'incorrect', 'wrong', 'negative'];
        
        if (!yesAnswers.includes(normalized) && !noAnswers.includes(normalized)) {
          // Add error message
          setTimeout(() => {
            setMsgs(prev => [...prev, {
              role: 'ai',
              text: 'Please answer with "Yes" or "No" (or similar like "yeah", "nope", etc.).'
            }]);
            setAiMessageAdded(true);
            
            // Restart voice recognition if in voice mode
            if (voiceMode && inputStartRef.current) {
              setTimeout(() => {
                console.log('Restarting voice recognition after validation error');
                inputStartRef.current?.();
              }, 500);
            }
          }, 500);
          return;
        }
        
        // Normalize the answer for storage
        if (yesAnswers.includes(normalized)) {
          text = 'Yes';
        } else {
          text = 'No';
        }
      }
    }
    
    setDraft('');
    
    // Add user message
    const conversation = [...msgsRef.current, { role: 'user', text }];
    setMsgs(conversation);
    msgsRef.current = conversation;

    // If inquiry is complete, handle final submission
    if (isComplete) {
      // Gather all stored answers
      const answers = [];
      questions.forEach(q => {
        const stored = sessionStorage.getItem(`inquiry_${productId}_${q.id}`);
        if (stored) {
          try {
            const data = JSON.parse(stored);
            answers.push(data.answer);
          } catch {
            answers.push('N/A');
          }
        } else {
          answers.push('N/A');
        }
      });

      // Prepare inquiry data for chat system
      const inquiryData = {
        productId,
        productTitle,
        questions,
        answers
      };

      // Store in sessionStorage for chat to pick up
      sessionStorage.setItem('pending-inquiry-message', JSON.stringify(inquiryData));

      // Clean up individual answer storage
      questions.forEach(q => {
        sessionStorage.removeItem(`inquiry_${productId}_${q.id}`);
      });

      setTimeout(() => {
        setMsgs(prev => [...prev, {
          role: 'ai',
          text: 'Thank you! Your inquiry has been sent to the seller. They will contact you soon through our chat system.'
        }]);
        setAiMessageAdded(true);
        
        // Redirect to chat after a delay
        setTimeout(() => {
          navigate('/chat');
        }, 3000);
      }, 500);
      return;
    }

    // Store the answer
    if (currentQuestionIndex < questions.length) {
      const currentQ = questions[currentQuestionIndex];
      // We can store answers in sessionStorage or send to API here
      sessionStorage.setItem(`inquiry_${productId}_${currentQ.id}`, JSON.stringify({
        question: currentQ.question,
        answer: text,
        question_type: currentQ.question_type
      }));

      // Move to next question or complete
      const nextIndex = currentQuestionIndex + 1;
      if (nextIndex < questions.length) {
        setCurrentQuestionIndex(nextIndex);
        const nextQ = questions[nextIndex];
        
        setTimeout(() => {
          setMsgs(prev => [...prev, {
            role: 'ai',
            text: nextQ.question + (nextQ.question_type === 'YesNo' ? ' (Yes/No)' : '')
          }]);
          setAiMessageAdded(true);
          
          // Restart voice recognition if in voice mode
          if (voiceMode && inputStartRef.current) {
            setTimeout(() => {
              console.log('Restarting voice recognition after AI response');
              inputStartRef.current?.();
            }, 500);
          }
        }, 1000);
      } else {
        // All questions answered
        setIsComplete(true);
        setTimeout(() => {
          setMsgs(prev => [...prev, {
            role: 'ai',
            text: 'Perfect! I have all the information I need. Would you like me to send this inquiry to the seller now? Just say "yes" or "send it".'
          }]);
          setAiMessageAdded(true);
          
          // Restart voice recognition if in voice mode
          if (voiceMode && inputStartRef.current) {
            setTimeout(() => {
              console.log('Restarting voice recognition after AI response');
              inputStartRef.current?.();
            }, 500);
          }
        }, 1000);
      }
    }
  };

  // Focus input after messages are added
  useEffect(() => {
    if (messages.length > 0 && inputRef.current && !voiceMode) {
      setTimeout(() => {
        if (inputRef.current) {
          inputRef.current.focus();
        }
      }, 100);
    }
  }, [messages.length, voiceMode]);

  // Start voice recognition after first question is loaded (separate from question loading)
  useEffect(() => {
    if (voiceMode && questions.length > 0 && messages.length === 1 && inputStartRef.current) {
      // Only start voice for the very first question, when we have exactly 1 message (the first AI question)
      const timer = setTimeout(() => {
        console.log('Starting voice recognition for first question');
        inputStartRef.current?.();
      }, 500); // Shorter delay since question is already rendered
      
      return () => clearTimeout(timer);
    }
  }, [voiceMode, questions.length, messages.length]);

  return (
    <div className="chat-wrap">
      <button className="back-btn" onClick={() => {
        // Stop voice recognition when closing
        if (voiceMode && voiceInputRef.current) {
          console.log('Stopping voice recognition on close');
          if (voiceInputRef.current.stop) {
            voiceInputRef.current.stop();
          }
        }
        onClose ? onClose() : navigate(-1);
      }}>{'Back'}</button>
      
      <div className="inquiry-header">
        <h3>Inquiring about: {productTitle}</h3>
        <div className="progress-bar">
          <div 
            className="progress-fill" 
            style={{ 
              width: `${((currentQuestionIndex + (isComplete ? 1 : 0)) / (questions.length + 1)) * 100}%` 
            }}
          />
        </div>
        <span className="progress-text">
          {isComplete ? 'Complete' : `Question ${currentQuestionIndex + 1} of ${questions.length}`}
        </span>
      </div>

      <div className="chat-window" ref={scrollRef}>
        {messages.map((m, i) => (
          <div key={i} className={`bubble ${m.role}`}>
            {m.text}
          </div>
        ))}
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
