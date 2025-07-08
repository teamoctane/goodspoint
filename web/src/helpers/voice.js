export const getSpeechRec = () => {
  const C = window.SpeechRecognition || window.webkitSpeechRecognition;
  return C ? new C() : null;
};

export async function whisperGroq(blob) {
  const fd = new FormData();
  fd.append('file', blob, 'audio.mp3');
  fd.append('model', 'whisper-large-v3');
  const res = await fetch('https://api.groq.com/v1/audio/transcriptions', {
    method: 'POST',
    headers: { Authorization: `Bearer ${process.env.REACT_APP_GROQ_KEY}` },
    body: fd,
  });
  const { text } = await res.json();
  return text;
} 