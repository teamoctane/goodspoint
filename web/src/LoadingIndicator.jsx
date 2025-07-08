import './App.css';

function LoadingIndicator({ inline = false }) {
  if (inline) {
    return (
      <div className="loading-inline">
        <div className="loading-dot"></div>
        <div className="loading-dot"></div>
        <div className="loading-dot"></div>
      </div>
    );
  }

  return (
    <div className="loading-overlay">
      <div className="loading-spinner">
        <div className="loading-dot"></div>
        <div className="loading-dot"></div>
        <div className="loading-dot"></div>
      </div>
    </div>
  );
}

export default LoadingIndicator;
