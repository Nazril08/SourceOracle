import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { FiSearch, FiRefreshCw, FiAlertCircle, FiTrash2, FiSettings } from 'react-icons/fi';
import { IoGameController } from 'react-icons/io5';
import useImagePreload from '../hooks/useImagePreload';
import { getGameImage } from '../utils/steamImages';
import DlcManager from './DlcManager';
import { FiCheckCircle, FiXCircle, FiDownload } from 'react-icons/fi';
import { BsChevronLeft, BsChevronRight } from 'react-icons/bs';

const STEAM_PATHS = {
  LUA_DIR: 'C:\\Program Files (x86)\\Steam\\config\\stplug-in',
  MANIFEST_DIR: 'C:\\Program Files (x86)\\Steam\\config\\depotcache'
};

const GameRow = ({ game, onUpdate, onRemove, onManageDlcs, shouldLoadImage, isUpdating }) => {
  const imageUrl = getGameImage(game.app_id, 'capsule');
  const { isLoaded, error } = useImagePreload(shouldLoadImage ? imageUrl : null);

  // Check if the name is just an AppID placeholder
  const isAppIdOnly = game.name.startsWith("AppID:");
  const displayName = isAppIdOnly ? "Unknown Game" : game.name;

  return (
    <div className="flex items-center bg-surface rounded-lg p-4 gap-4 backdrop-blur-sm bg-opacity-80 border border-border">
      {/* Game Image */}
      <div className="w-48 h-24 bg-sidebar rounded-md overflow-hidden flex-shrink-0">
        {shouldLoadImage ? (
          !error ? (
            <img
              src={imageUrl}
              alt={displayName}
              className={`w-full h-full object-cover transition-opacity duration-300 ${
                isLoaded ? 'opacity-100' : 'opacity-0'
              }`}
            />
          ) : (
            <div className="w-full h-full flex items-center justify-center bg-sidebar">
              <IoGameController className="text-primary text-4xl" />
            </div>
          )
        ) : (
          <div className="w-full h-full flex items-center justify-center bg-sidebar">
            <IoGameController className="text-primary text-4xl" />
          </div>
        )}
      </div>

      {/* Game Info */}
      <div className="flex-1">
        <h3 className="text-lg font-bold text-white">{displayName}</h3>
        <p className="text-gray-400 text-sm">AppID: {game.app_id}</p>
        <div className="mt-2 flex gap-2 text-sm">
          <span className={`px-2 py-1 rounded ${game.lua_file ? 'bg-green-900 text-green-300' : 'bg-red-900 text-red-300'}`}>
            LUA {game.lua_file ? '✓' : '✗'}
          </span>
          <span className={`px-2 py-1 rounded ${game.manifest_file ? 'bg-green-900 text-green-300' : 'bg-red-900 text-red-300'}`}>
            Manifest {game.manifest_file ? '✓' : '✗'}
          </span>
        </div>
      </div>

      {/* Action Buttons */}
      <div className="flex gap-2">
        <button
          onClick={() => onUpdate(game)}
          disabled={isUpdating}
          className="px-4 py-2 bg-primary hover:bg-highlight rounded-md transition-colors duration-200 disabled:bg-gray-500 disabled:cursor-not-allowed"
        >
          {isUpdating ? 'Updating...' : 'Update'}
        </button>
        <button
          onClick={() => onManageDlcs(game)}
          className="px-4 py-2 bg-blue-600 hover:bg-blue-700 rounded-md transition-colors duration-200 flex items-center gap-1"
        >
          <FiSettings />
          <span>DLCs</span>
        </button>
        <button
          onClick={() => onRemove(game)}
          className="px-4 py-2 bg-red-600 hover:bg-red-700 rounded-md transition-colors duration-200 flex items-center gap-1"
        >
          <FiTrash2 />
          <span>Remove</span>
        </button>
      </div>
    </div>
  );
};

const MyLibrary = ({ showNotification }) => {
  const [games, setGames] = useState([]);
  const [filteredGames, setFilteredGames] = useState([]);
  const [searchQuery, setSearchQuery] = useState('');
  const [searchInputValue, setSearchInputValue] = useState('');
  const [currentPage, setCurrentPage] = useState(1);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState(null);
  const [directoryStatus, setDirectoryStatus] = useState({
    lua: false,
    manifest: false
  });
  const [confirmRemove, setConfirmRemove] = useState(null);
  const [updatingGames, setUpdatingGames] = useState({});
  const [managingDlcsFor, setManagingDlcsFor] = useState(null);
  const gamesPerPage = 10;

  const checkDirectories = async () => {
    try {
      const status = await invoke('check_steam_directories', {
        luaPath: STEAM_PATHS.LUA_DIR,
        manifestPath: STEAM_PATHS.MANIFEST_DIR
      });
      setDirectoryStatus(status);
    } catch (err) {
      console.error('Failed to check directories:', err);
      setError('Failed to access Steam directories. Please check if Steam is installed correctly.');
    }
  };

  const loadLibrary = async () => {
    setIsLoading(true);
    setError(null);
    
    try {
      // First check if directories exist
      await checkDirectories();

      const libraryGames = await invoke('get_library_games', {
        luaDir: STEAM_PATHS.LUA_DIR,
        manifestDir: STEAM_PATHS.MANIFEST_DIR
      });

      // Add file status to each game
      const gamesWithStatus = libraryGames.map(game => ({
        ...game,
        lua_file: game.lua_file || false,
        manifest_file: game.manifest_file || false
      }));

      setGames(gamesWithStatus);
      
      // Only apply search filter if there's an active search query
      if (searchQuery) {
        filterGames(gamesWithStatus, searchQuery);
      } else {
        setFilteredGames(gamesWithStatus);
      }
    } catch (err) {
      setError(err.toString());
      setGames([]);
      setFilteredGames([]);
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    const startup = async () => {
      try {
        console.log('Clearing old details cache to ensure data is fresh...');
        await invoke('clear_details_cache');
        showNotification('Refreshed app cache to load new data.', 'info');
      } catch (err) {
        console.error('Failed to clear details cache:', err);
        // Do not show an error notification for this, it is not a critical failure.
      } finally {
        // Always load the library afterwards.
    loadLibrary();
      }
    };

    startup();
  }, []); // Empty dependency array ensures this runs only once on mount.

  // Update searchInputValue when searchQuery changes
  useEffect(() => {
    setSearchInputValue(searchQuery);
  }, [searchQuery]);

  // Fetch game names for any games that only have AppIDs
  useEffect(() => {
    const fetchMissingGameNames = async () => {
      if (!games.length) return;
      
      const gamesWithMissingNames = games.filter(game => game.name.startsWith("AppID:"));
      if (!gamesWithMissingNames.length) return;
      
      console.log(`Fetching names for ${gamesWithMissingNames.length} games...`);
      
      const updatedGames = [...games];
      
      for (const game of gamesWithMissingNames) {
        try {
          const gameName = await invoke('get_game_name_by_appid', { appId: game.app_id });
          
          // Update the game in our local state
          const index = updatedGames.findIndex(g => g.app_id === game.app_id);
          if (index !== -1) {
            updatedGames[index] = {
              ...updatedGames[index],
              name: gameName
            };
          }
        } catch (err) {
          console.error(`Failed to fetch name for game ${game.app_id}:`, err);
        }
      }
      
      setGames(updatedGames);
      
      // Only apply search filter if there's an active search query
      if (searchQuery) {
        filterGames(updatedGames, searchQuery);
      } else {
        setFilteredGames(updatedGames);
      }
    };
    
    fetchMissingGameNames();
  }, [games.length, searchQuery]);

  const filterGames = (gameList, query) => {
    if (!query) {
      setFilteredGames(gameList);
      setCurrentPage(1);
      return;
    }
    
    const lowercaseQuery = query.toLowerCase();
    const filtered = gameList.filter(game => {
      // Check if the name is just an AppID placeholder
      const isAppIdOnly = game.name.startsWith("AppID:");
      
      // If it's just an AppID placeholder, only search in the AppID
      if (isAppIdOnly) {
        return game.app_id.toString().includes(lowercaseQuery);
      }
      
      // Otherwise search in both name and AppID
      return (
        game.name.toLowerCase().includes(lowercaseQuery) ||
        game.app_id.toString().includes(lowercaseQuery)
      );
    });
    
    setFilteredGames(filtered);
    setCurrentPage(1);
  };

  const handleSearchInputChange = (e) => {
    // Only update the input value, don't trigger search
    setSearchInputValue(e.target.value);
  };

  const handleSearchSubmit = (e) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      setSearchQuery(searchInputValue);
      filterGames(games, searchInputValue);
    }
  };

  const handleUpdate = async (game) => {
    setUpdatingGames(prev => ({ ...prev, [game.app_id]: true }));
    setError(null);
    try {
      const result = await invoke('update_game_files', {
        appId: game.app_id,
        gameName: game.name
      });
      showNotification(result, 'success');
      // Refresh library to show updated status
      loadLibrary();
    } catch (err) {
      const errorMessage = `Failed to update ${game.name}: ${err.toString()}`;
      setError(errorMessage);
      showNotification(errorMessage, 'error');
    } finally {
      setUpdatingGames(prev => ({ ...prev, [game.app_id]: false }));
    }
  };

  const handleRemove = (game) => {
    // Set the game to be confirmed for removal
    setConfirmRemove(game);
  };

  const confirmRemoveGame = async () => {
    if (!confirmRemove) return;
    
    try {
      await invoke('remove_game', { appId: confirmRemove.app_id });
      // Refresh library after removal
      loadLibrary();
    } catch (err) {
      setError(err.toString());
    } finally {
      // Clear the confirmation
      setConfirmRemove(null);
    }
  };

  const cancelRemove = () => {
    setConfirmRemove(null);
  };

  const handleRefresh = () => {
    loadLibrary();
  };

  const handleManageDlcs = (game) => {
    setManagingDlcsFor(game);
  };

  // Calculate pagination
  const totalPages = Math.ceil(filteredGames.length / gamesPerPage);
  const startIndex = (currentPage - 1) * gamesPerPage;
  const endIndex = startIndex + gamesPerPage;
  const currentGames = filteredGames.slice(startIndex, endIndex);

  // Pagination controls
  const handlePageChange = (newPage) => {
    if (newPage >= 1 && newPage <= totalPages) {
      setCurrentPage(newPage);
      // Scroll to top when page changes
      const container = document.querySelector('.overflow-auto');
      if (container) {
        container.scrollTop = 0;
      }
    }
  };

  return (
    <div className="flex-1 overflow-hidden flex flex-col">
      {/* DLC Manager Modal */}
      {managingDlcsFor && (
        <DlcManager
          game={managingDlcsFor}
          onClose={() => setManagingDlcsFor(null)}
          showNotification={showNotification}
        />
      )}

      {/* Confirmation Modal */}
      {confirmRemove && (
        <div className="fixed inset-0 bg-black bg-opacity-70 flex items-center justify-center z-50">
          <div className="bg-surface p-6 rounded-lg max-w-md w-full border border-border">
            <h3 className="text-xl font-bold mb-4">Confirm Removal</h3>
            <p className="mb-6">
              Are you sure you want to remove <span className="font-bold text-primary">{confirmRemove.name}</span> (AppID: {confirmRemove.app_id}) from your library?
            </p>
            <div className="flex justify-end gap-3">
              <button 
                onClick={cancelRemove}
                className="px-4 py-2 bg-sidebar hover:bg-sidebar-active rounded-md transition-colors duration-200"
              >
                Cancel
              </button>
              <button 
                onClick={confirmRemoveGame}
                className="px-4 py-2 bg-red-600 hover:bg-red-700 rounded-md transition-colors duration-200"
              >
                Remove
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Header */}
      <div className="p-6">
        <div className="flex justify-between items-center mb-4">
          <div>
            <h1 className="text-3xl font-bold">My Library</h1>
            <div className="flex gap-4 mt-2">
              <span className={`text-sm ${directoryStatus.lua ? 'text-green-400' : 'text-red-400'}`}>
                <FiAlertCircle className="inline mr-1" />
                LUA Directory: {directoryStatus.lua ? 'Found' : 'Not Found'}
              </span>
              <span className={`text-sm ${directoryStatus.manifest ? 'text-green-400' : 'text-red-400'}`}>
                <FiAlertCircle className="inline mr-1" />
                Manifest Directory: {directoryStatus.manifest ? 'Found' : 'Not Found'}
              </span>
            </div>
          </div>
          <button
            onClick={handleRefresh}
            className="px-4 py-2 bg-primary hover:bg-highlight rounded-md transition-colors duration-200 flex items-center gap-2"
          >
            <FiRefreshCw />
            REFRESH
          </button>
        </div>

        <p className="text-gray-400 mb-4">
          Showing all {filteredGames.length} games in your library.
        </p>

        {/* Search Bar */}
        <div className="relative">
          <FiSearch 
            className="absolute left-3 top-1/2 transform -translate-y-1/2 text-gray-400 cursor-pointer" 
            onClick={handleSearchSubmit}
          />
          <input
            type="text"
            value={searchInputValue}
            onChange={handleSearchInputChange}
            onKeyDown={handleSearchSubmit}
            placeholder="Search games... (press Enter to search)"
            className="w-full pl-10 pr-4 py-2 bg-sidebar border border-border rounded-md focus:ring-primary focus:border-primary"
          />
        </div>
      </div>

      {/* Directory Error Message */}
      {(!directoryStatus.lua || !directoryStatus.manifest) && (
        <div className="mx-6 p-4 bg-red-900 bg-opacity-50 rounded-lg border border-red-700">
          <h3 className="text-red-400 font-bold flex items-center gap-2">
            <FiAlertCircle />
            Steam Directory Error
          </h3>
          <p className="mt-2 text-sm text-red-300">
            Some Steam directories were not found. Please check if Steam is installed correctly at:
          </p>
          <ul className="mt-1 text-sm text-red-300 list-disc list-inside">
            <li>LUA files: {STEAM_PATHS.LUA_DIR}</li>
            <li>Manifest files: {STEAM_PATHS.MANIFEST_DIR}</li>
          </ul>
        </div>
      )}

      {/* Error Message */}
      {error && (
        <div className="px-6 py-2 text-red-400">
          Error: {error}
        </div>
      )}

      {/* Game List */}
      <div className="flex-1 overflow-auto px-6">
        <div className="space-y-4">
          {isLoading ? (
            <div className="flex items-center justify-center py-8">
              <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"></div>
            </div>
          ) : currentGames.length > 0 ? (
            currentGames.map((game, index) => (
              <GameRow
                key={game.app_id}
                game={game}
                onUpdate={handleUpdate}
                onRemove={handleRemove}
                onManageDlcs={handleManageDlcs}
                isUpdating={updatingGames[game.app_id]}
                shouldLoadImage={index < gamesPerPage}
              />
            ))
          ) : (
            <div className="text-center py-8 text-gray-400">
              No games found in your library.
            </div>
          )}
        </div>
      </div>

      {/* Pagination */}
      {totalPages > 1 && (
        <div className="p-4 border-t border-border flex justify-center">
          <div className="flex gap-2">
            <button
              onClick={() => handlePageChange(1)}
              disabled={currentPage === 1}
              className={`px-3 py-1 rounded ${
                currentPage === 1 ? 'bg-sidebar text-gray-500' : 'bg-primary hover:bg-highlight'
              }`}
            >
              First
            </button>
            <button
              onClick={() => handlePageChange(currentPage - 1)}
              disabled={currentPage === 1}
              className={`px-3 py-1 rounded ${
                currentPage === 1 ? 'bg-sidebar text-gray-500' : 'bg-primary hover:bg-highlight'
              }`}
            >
              Prev
            </button>
            
            <div className="flex items-center px-4">
              <span className="text-sm">
                Page {currentPage} of {totalPages}
              </span>
            </div>
            
            <button
              onClick={() => handlePageChange(currentPage + 1)}
              disabled={currentPage === totalPages}
              className={`px-3 py-1 rounded ${
                currentPage === totalPages ? 'bg-sidebar text-gray-500' : 'bg-primary hover:bg-highlight'
              }`}
            >
              Next
            </button>
            <button
              onClick={() => handlePageChange(totalPages)}
              disabled={currentPage === totalPages}
              className={`px-3 py-1 rounded ${
                currentPage === totalPages ? 'bg-sidebar text-gray-500' : 'bg-primary hover:bg-highlight'
              }`}
            >
              Last
            </button>
          </div>
        </div>
      )}
    </div>
  );
};

export default MyLibrary; 