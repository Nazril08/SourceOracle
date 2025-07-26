import { useState, useEffect } from 'react';

const useImagePreload = (imageUrl) => {
  const [isLoaded, setIsLoaded] = useState(false);
  const [error, setError] = useState(null);

  useEffect(() => {
    // Reset states when imageUrl changes
    setIsLoaded(false);
    setError(null);
    
    // If no URL provided, don't attempt to load
    if (!imageUrl) {
      return;
    }

    const img = new Image();
    
    img.onload = () => {
      setIsLoaded(true);
    };

    img.onerror = () => {
      setError('Failed to load image');
    };

    img.src = imageUrl;

    return () => {
      img.onload = null;
      img.onerror = null;
    };
  }, [imageUrl]);

  return { isLoaded, error };
};

export default useImagePreload; 