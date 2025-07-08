import { useEffect, useRef, useState, useCallback, useImperativeHandle, forwardRef } from 'react';
import { getSpeechRec } from './helpers/voice';
import { useLang, LANGS, T } from './i18n';

const SILENCE_MS = 1500;         // 1.5 s of silence

export function TextInputBar({ onSend, value, setValue, inputRef }){
  const handleSend = () => {
    const text = value.trim();
    if (text) {
      onSend(text);
      setValue(''); // Clear input after sending
    }
  };

  const handleKeyDown = (e) => {
    if (e.key === 'Enter') {
      handleSend();
    }
  };

  return (
    <div className="input-bar">
      <div className="textbox centre">
        <input 
          ref={inputRef}
          className="input"
          placeholder={'Describe what you want…'}
          value={value}
          onChange={e=>setValue(e.target.value)}
          onKeyDown={handleKeyDown}
        />
        <button className="submit" onClick={handleSend}>
          <span className="material-symbols-outlined">arrow_forward</span>
        </button>
      </div>
    </div>
  );
}

export const VoiceInputBar = forwardRef(({ onSend, autoStart, getStartRef }, ref) => {
  const [draft, setDraft] = useState('');
  const [listening, setL] = useState(false);
  const { lang } = useLang();
  
  const listeningText = 'Listening…';           // Listening…
  const idleText = '< Click button to speak';// idle prompt

  const recRef = useRef(null);
  const mediaRef = useRef(null);
  const streamRef = useRef(null); // Track the media stream
  const chunksRef = useRef([]);
  const silentTimer = useRef(null);
  const alreadySent = useRef(false);
  const draftRef = useRef('');  // ← add
  const isStarting = useRef(false); // Track if we're in the process of starting
  const micRef = useRef(null);



  /*──────── helpers ────────*/
  const clearTimer = () => clearTimeout(silentTimer.current);

  const send = useCallback(txt => {
    const t = txt.trim();
    if (!t) return;
    alreadySent.current = true;
    setDraft('');  draftRef.current = '';       // keep state & ref in sync
    onSend(t);
    // Don't restart - let the user manually start again if needed
  }, [onSend]);

  const stop = useCallback(async (fromSilence=false) => {
    clearTimer();
    if (recRef.current) {
      try {
        recRef.current.stop();
      } catch (e) {
        // Ignore errors when stopping
      }
    }
    if (mediaRef.current) {
      try {
        mediaRef.current.stop();
      } catch (e) {
        // Ignore errors when stopping
      }
    }
    // Stop the media stream to turn off microphone indicator
    if (streamRef.current) {
      try {
        streamRef.current.getTracks().forEach(track => track.stop());
        streamRef.current = null;
      } catch (e) {
        // Ignore errors when stopping stream
      }
    }

    /* Auto-send once */
    if (fromSilence && !alreadySent.current) {
      const txt = draftRef.current.trim();     // <-- guaranteed value
      if (txt) send(txt);
      draftRef.current = '';                   // reset cache
    }
    setL(false);
    isStarting.current = false; // Reset starting flag
  }, [send]);

  // Expose stop method to parent through ref
  useImperativeHandle(ref, () => ({
    stop: () => stop(false)
  }), [stop]);

  const scheduleSend = useCallback(() => {
    clearTimer();
    silentTimer.current = setTimeout(() => stop(true), SILENCE_MS);
  }, [stop]);

  /*──────── mic control ─────*/
  const start = useCallback(() => {
    // Prevent multiple simultaneous starts
    if (isStarting.current || listening) return;
    
    const rec = recRef.current ?? (recRef.current = getSpeechRec());
    if (!rec) return alert('Web Speech API not supported 😀');

    isStarting.current = true;
    alreadySent.current = false;
    setDraft(''); 
    setL(true);

    /* pick language from context */
    rec.lang = lang === LANGS.EN ? 'en-US' : 'hi-IN';
    rec.continuous = true;
    rec.interimResults = true;

    rec.onresult = e => {
      const res = e.results[e.resultIndex];
      if (res.isFinal) {
        draftRef.current = res[0].transcript;  // cache immediately
        setDraft(draftRef.current);            // update UI
        scheduleSend();                        // debounce
      }
    };

    rec.onerror = (event) => {
      console.log('Speech recognition error:', event.error);
      stop(false);
    };

    rec.onend = () => {
      // Only reset if we're not intentionally stopping
      if (listening && !isStarting.current) {
        console.log('Speech recognition ended unexpectedly');
      }
    };

    try {
      rec.start();
    } catch (error) {
      console.error('Error starting speech recognition:', error);
      isStarting.current = false;
      setL(false);
      return;
    }

    navigator.mediaDevices.getUserMedia({ audio:true }).then(s=>{
      streamRef.current = s; // Store the stream reference
      const mr = new MediaRecorder(s, { mimeType:'audio/webm' });
      mr.ondataavailable = d => chunksRef.current.push(d.data);
      mr.start();
      mediaRef.current = mr;
    }).catch(error => {
      console.error('Error getting user media:', error);
    });
  }, [lang, scheduleSend, stop, listening]);

  /*──────── autoStart from /search?voiceMode ────*/
  useEffect(() => {
    if (autoStart && !listening && !isStarting.current) {
      // Only auto-start for SearchChat (not InquiryChat which handles its own voice start)
      const isInquiryOverlay = document.querySelector('.inquiry-overlay');
      if (!isInquiryOverlay) {
        // Only auto-start if it's the initial mount, not after messages are added
        const hasMessages = document.querySelector('.bubble');
        if (!hasMessages) {
          // Add a small delay to ensure previous recognition is fully stopped
          setTimeout(() => start(), 100);
        }
      }
    }
  }, [autoStart, listening, start]);

  /*──────── expose start method to parent ────*/
  useEffect(() => {
    if (getStartRef) getStartRef(() => start());
  }, [getStartRef, start]);



  // Cleanup on unmount
  useEffect(() => {
    return () => {
      clearTimer();
      if (recRef.current) {
        try {
          recRef.current.stop();
        } catch (e) {
          // Ignore cleanup errors
        }
      }
      if (mediaRef.current) {
        try {
          mediaRef.current.stop();
        } catch (e) {
          // Ignore cleanup errors
        }
      }
      // Stop the media stream on cleanup
      if (streamRef.current) {
        try {
          streamRef.current.getTracks().forEach(track => track.stop());
          streamRef.current = null;
        } catch (e) {
          // Ignore cleanup errors
        }
      }
    };
  }, []);

  return (
    <div className="voice-bar">
      <div className="voice-inner">
        <button
          className={`circle mic-btn${listening?' live':''}`}
          ref={micRef}
          onClick={listening ? () => stop(false) : start}
        >
          <span className="material-symbols-outlined">
            {listening ? 'stop' : 'mic'}
          </span>
        </button>
        <div className={`textbox centre ${listening?'glow speak':''}`}>
          <input className="input"
                 readOnly
                 placeholder={listening ? listeningText : idleText}
                 value={draft}/>
        </div>
      </div>
    </div>
  );
});

 