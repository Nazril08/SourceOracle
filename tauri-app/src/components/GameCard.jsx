import React, { useState } from 'react';
import { IoGameController } from 'react-icons/io5';
import { invoke } from '@tauri-apps/api/tauri';

const GameCard = ({ game, isLoading, onShowDetails }) => {
  const [loadingDetails, setLoadingDetails] = useState(false);
  const [message, setMessage] = useState('');
  const [imageError, setImageError] = useState(false);

  const handleDetailClick = async () => {
    setLoadingDetails(true);
    setMessage('Loading details...');
    
    try {
      const gameDetails = await invoke('get_game_details', { 
        appId: game.app_id 
      });
      
      if (onShowDetails) {
        onShowDetails(gameDetails);
      }
      
      setMessage('');
    } catch (error) {
      console.error(`Failed to fetch details for ${game.app_id}:`, error);
      setMessage(`Error: Could not load details.`);
    } finally {
      setLoadingDetails(false);
    }
  };

  const handleImageError = () => {
    setImageError(true);
  };

  return (
    <div className="bg-surface rounded-lg backdrop-blur-sm bg-opacity-80 shadow-card border border-border h-[280px]">
      <div className="p-4 flex flex-col h-full">
        {/* Game Image */}
        <div className="w-full h-32 mb-3 overflow-hidden rounded-md bg-sidebar flex items-center justify-center">
          {!imageError && game.icon_url ? (
            <img 
              src={game.icon_url} 
              alt={game.game_name}
              className="w-full h-full object-cover"
              onError={handleImageError}
            />
          ) : (
            <IoGameController className="text-primary text-5xl" />
          )}
        </div>
        
        {/* Game Info */}
        <div className="flex flex-col justify-between flex-1">
          <div>
            <h3 className="text-lg font-bold truncate" title={game.game_name}>
              {game.game_name}
            </h3>
            <p className="text-gray-400 text-sm mt-1">AppID: {game.app_id}</p>
            
            {message && (
              <p className={`mt-2 text-sm ${
                message.startsWith('Error') ? 'text-red-400' : 'text-gray-300'
              }`}>
                {message}
              </p>
            )}
          </div>
          
          <div className="mt-3">
            <button 
              onClick={handleDetailClick}
              disabled={isLoading || loadingDetails}
              className="w-full px-4 py-2 bg-primary hover:bg-highlight rounded-md transition-colors duration-200 disabled:opacity-50"
            >
              {loadingDetails ? 'Loading...' : 'Detail Game'}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};

export default GameCard; 