import { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { listen } from '@tauri-apps/api/event';
import { FiSearch, FiSettings, FiRefreshCw, FiArrowLeft, FiAlertTriangle, FiDownload, FiBook, FiUsers, FiEdit, FiX, FiHome, FiBookOpen, FiInfo, FiFileText } from 'react-icons/fi';
import { IoGameController } from 'react-icons/io5';
import { BsChevronLeft, BsChevronRight } from 'react-icons/bs';
import GameCard from './components/GameCard';
import SettingsPanel from './components/SettingsPanel';
import MyLibrary from './components/MyLibrary';
import AccountManager from './components/AccountManager';
import StartupLoader from './components/StartupLoader';
import useImagePreload from './hooks/useImagePreload';
import { getGameImage } from './utils/steamImages';
import DlcManager from './components/DlcManager';
import NotesManager from './components/NotesManager';

function App() {
  const [isInitialized, setIsInitialized] = useState(false);
  const [activeTab, setActiveTab] = useState('game');
  const [searchQuery, setSearchQuery] = useState('');
  const [accounts, setAccounts] = useState([]); // State for account sharing
  const [accountSearchQuery, setAccountSearchQuery] = useState('');
  const [accountCurrentPage, setAccountCurrentPage] = useState(1);
  const accountsPerPage = 20;
  const [searchResults, setSearchResults] = useState({
    games: [],
    total: 0,
    page: 1,
    total_pages: 1,
    query: ''
  });
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState(null);
  const [notification, setNotification] = useState(null);
  const [currentPage, setCurrentPage] = useState(1);
  const [selectedGame, setSelectedGame] = useState(null);
  const [showingDetails, setShowingDetails] = useState(false);
  const [isDownloading, setIsDownloading] = useState(false);
  const [downloadStatus, setDownloadStatus] = useState('idle'); // idle, downloading, success, error
  const [isManagingAccounts, setIsManagingAccounts] = useState(false); // State to control the modal
  const [view, setView] = useState('library');

  // Ref to track the current search request
  const searchRequestRef = useRef(0);

  // Get header image URL based on AppID
  const headerImageUrl = selectedGame ? getGameImage(selectedGame.app_id || selectedGame.steam_appid, 'header') : null;
  
  // Preload the header image
  const { isLoaded, error: imageError } = useImagePreload(headerImageUrl);

  useEffect(() => {
    // This effect runs only once to set up the initialization listener.
    const unlisten = listen('initialization_complete', () => {
      console.log('Backend initialization complete. Loading UI data...');
      // Now that backend is ready, fetch all necessary data for the UI.
      loadInitialData();
    });

    async function loadInitialData() {
        // We use Promise.all to run these in parallel for faster loading.
        await Promise.all([
            loadAccounts(),
            handleSearch('', 1) // Load initial games for 'Explore' tab
        ]);
        // Once all data is loaded, unmount the loader and show the app.
        setIsInitialized(true);
    }

    // Cleanup listener on component unmount
    return () => {
      unlisten.then(f => f());
    };
  }, []); // Empty dependency array means this runs once on mount.

  // Effect to preload image when game is selected
  useEffect(() => {
    // No need to do anything special here, the useImagePreload hook will handle image changes
    // The previous code was trying to access an undefined 'imagePreload' variable
  }, [selectedGame]);

  const loadAccounts = async () => {
    try {
      const fetchedAccounts = await invoke('get_accounts');
      setAccounts(fetchedAccounts);
    } catch (err) {
      showNotification(`Error loading accounts: ${err.toString()}`, 'error');
    }
  };

  const handleSwitchAccount = async (account) => {
    showNotification(`Switching to ${account.displayName}...`, 'info');
    try {
      await invoke('switch_steam_account', {
        username: account.steamUsername,
        password: account.steamPassword,
      });
      showNotification(`Successfully launched Steam for ${account.displayName}`, 'success');
    } catch (err) {
      showNotification(`Failed to switch account: ${err.toString()}`, 'error');
    }
  };

  const handleSaveAccounts = (updatedAccounts) => {
    setAccounts(updatedAccounts);
    showNotification('Account list updated successfully!', 'success');
  };

  // Filtering logic for account sharing
  const filteredAccounts = accounts.filter(account =>
    account.displayName.toLowerCase().includes(accountSearchQuery.toLowerCase()) ||
    account.gameName.toLowerCase().includes(accountSearchQuery.toLowerCase())
  );

  // Pagination logic for account sharing
  const indexOfLastAccount = accountCurrentPage * accountsPerPage;
  const indexOfFirstAccount = indexOfLastAccount - accountsPerPage;
  const currentAccounts = filteredAccounts.slice(indexOfFirstAccount, indexOfLastAccount);
  const totalAccountPages = Math.ceil(filteredAccounts.length / accountsPerPage);

  const handleAccountPageChange = (newPage) => {
    if (newPage < 1 || newPage > totalAccountPages) return;
    setAccountCurrentPage(newPage);
  };

  // Function to extract AppID from Steam store URL
  const extractAppIdFromUrl = (url) => {
    // Check if the input is a Steam store URL
    const steamUrlRegex = /https?:\/\/store\.steampowered\.com\/app\/(\d+)/i;
    const match = url.match(steamUrlRegex);
    
    if (match && match[1]) {
      return match[1]; // Return the AppID
    }
    
    return url; // Return the original input if it's not a Steam URL
  };

  const handleSearch = async (query, page, isNewSearch = false) => {
    // Increment the request counter
    const currentSearchId = ++searchRequestRef.current;
    
    setIsLoading(true);
    setError(null);
    
    try {
      // Create a timeout promise
      const timeoutPromise = new Promise((_, reject) => 
        setTimeout(() => reject(new Error('Query Gagal: Tidak ada di DataBase')), 5000)
      );

      // Race the search against the timeout
      const results = await Promise.race([
        invoke('search_games', { 
          query, 
          page: page, 
          perPage: 10 
        }),
        timeoutPromise
      ]);
      
      // If a new search has been started, ignore the results of this one
      if (currentSearchId !== searchRequestRef.current) {
        return;
      }

      setSearchResults(results);
      setCurrentPage(page);
      
      if (isNewSearch) {
        if (results.total > 0) {
          showNotification(`Found ${results.total} games matching "${query}"`, 'success');
        } else {
          showNotification(`No games found matching "${query}"`, 'info');
        }
      }
    } catch (err) {
      // If a new search has been started, don't show an error for the old one
      if (currentSearchId !== searchRequestRef.current) {
        return;
      }
      setError(err.toString());
      showNotification(`Error: ${err.toString()}`, 'error');
    } finally {
      // Only stop loading if this is the latest search request
      if (currentSearchId === searchRequestRef.current) {
        setIsLoading(false);
      }
    }
  };

  const handlePageChange = (newPage) => {
    if (newPage < 1 || newPage > searchResults.total_pages) return;
    handleSearch(searchResults.query, newPage, false);
  };

  const handleShowDetails = (gameDetails) => {
    // Use the processed details directly
    setSelectedGame(gameDetails);
    setShowingDetails(true);
  };

  const handleBackToSearch = () => {
    setShowingDetails(false);
    setSelectedGame(null);
  };

  const handleDownload = async (game) => {
    if (isDownloading) return;
    
    setIsDownloading(true);
    setDownloadStatus('downloading');
    setError(null);
    
    try {
      const downloadResult = await invoke('download_game', { 
        appId: (game.app_id || game.steam_appid).toString(), 
        gameName: game.name || game.game_name,
        outputDir: null // Passing null to use the saved settings
      });
      
      if (downloadResult) {
        showNotification(`Download completed for ${game.name || game.game_name}`, 'success');
        setDownloadStatus('success');
      } else {
        showNotification(`Data for ${game.name || game.game_name} not found in repositories.`, 'error');
        setDownloadStatus('error');
      }
      return downloadResult;
    } catch (err) {
      setError(err.toString());
      showNotification(`Download failed: ${err.toString()}`, 'error');
      setDownloadStatus('error');
      throw err;
    } finally {
      setTimeout(() => {
        setIsDownloading(false);
        setDownloadStatus('idle');
      }, 3000); // Reset after 3 seconds
    }
  };

  const handleRestartSteam = async () => {
    try {
      await invoke('restart_steam');
      showNotification('Steam has been restarted', 'success');
    } catch (err) {
      setError(err.toString());
      showNotification(`Failed to restart Steam: ${err.toString()}`, 'error');
    }
  };

  const handleInstallSteamTools = async () => {
    try {
      await invoke('install_steam_tools');
      showNotification('Starting SteamTools installation...', 'success');
    } catch (err) {
      setError(err.toString());
      showNotification(`Failed to start installation: ${err.toString()}`, 'error');
    }
  };

  const showNotification = (message, type = 'info') => {
    setNotification({ message, type });
    
    // Auto-hide notification after 5 seconds
    setTimeout(() => {
      setNotification(null);
    }, 5000);
  };

  const closeNotification = () => {
    setNotification(null);
  };

  const formatPrice = (priceOverview) => {
    if (!priceOverview) return 'Free';
    
    if (priceOverview.discount_percent > 0) {
      return `${priceOverview.final_formatted} (${priceOverview.discount_percent}% off)`;
    }
    
    return priceOverview.final_formatted;
  };

  const handleViewChange = (newView) => {
    setView(newView);
  };

  if (!isInitialized) {
    return <StartupLoader />;
  }

  return (
    <div className="flex h-screen bg-background text-white overflow-hidden">
      {/* Sidebar */}
      <div className="w-48 bg-sidebar flex flex-col border-r border-border">
        {/* Logo and Navigation */}
        <div className="flex-1">
          <div className="p-6">
            <h1 className="text-3xl font-bold text-primary">Yeyo</h1>
          </div>
          
          <nav className="mt-6">
            <button 
              onClick={() => setActiveTab('game')}
              className={`flex items-center w-full px-6 py-3 text-left ${
                activeTab === 'game' 
                  ? 'bg-sidebar-active text-white' 
                  : 'text-gray-400 hover:bg-opacity-50 hover:bg-sidebar-active hover:text-gray-200'
              }`}
            >
              <IoGameController className="mr-3 text-xl" />
              <span>Game</span>
            </button>
            
            <button
              onClick={() => setActiveTab('sharing')}
              className={`flex items-center w-full px-6 py-3 text-left ${
                activeTab === 'sharing'
                  ? 'bg-sidebar-active text-white'
                  : 'text-gray-400 hover:bg-opacity-50 hover:bg-sidebar-active hover:text-gray-200'
              }`}
            >
              <FiUsers className="mr-3" />
              <span>Account Sharing</span>
            </button>
            
            <button 
              onClick={() => setActiveTab('library')}
              className={`flex items-center w-full px-6 py-3 text-left ${
                activeTab === 'library' 
                  ? 'bg-sidebar-active text-white' 
                  : 'text-gray-400 hover:bg-opacity-50 hover:bg-sidebar-active hover:text-gray-200'
              }`}
            >
              <FiBook className="mr-3" />
              <span>Library</span>
            </button>
            
            <button 
              onClick={() => setActiveTab('notes')}
              className={`flex items-center w-full px-6 py-3 text-left ${
                activeTab === 'notes'
                  ? 'bg-sidebar-active text-white'
                  : 'text-gray-400 hover:bg-opacity-50 hover:bg-sidebar-active hover:text-gray-200'
              }`}
            >
              <FiFileText className="mr-3" />
              <span>Notes</span>
            </button>
            
            <button 
              onClick={() => setActiveTab('settings')}
              className={`flex items-center w-full px-6 py-3 text-left ${
                activeTab === 'settings' 
                  ? 'bg-sidebar-active text-white' 
                  : 'text-gray-400 hover:bg-opacity-50 hover:bg-sidebar-active hover:text-gray-200'
              }`}
            >
              <FiSettings className="mr-3" />
              <span>Settings</span>
            </button>
          </nav>
        </div>

        {/* Bottom Buttons */}
        <div className="p-4 space-y-2">
          <a href="#" onClick={handleInstallSteamTools} className="flex items-center space-x-3 p-2 rounded-lg hover:bg-gray-700 text-gray-300">
            <FiDownload className="h-6 w-6" />
            <span>Install SteamTools</span>
          </a>
          <button
            onClick={handleRestartSteam}
            className="w-full flex items-center justify-center px-4 py-2 text-sm font-medium text-white bg-red-600 rounded-md hover:bg-red-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-red-500"
          >
            <FiRefreshCw className="mr-2" />
            Restart Steam
          </button>
        </div>
      </div>

      {/* Main Content */}
      <main className="flex-1 flex flex-col overflow-hidden">
        {activeTab === 'game' && (
          <div className="flex-1 flex flex-col overflow-hidden">
            {showingDetails && selectedGame ? (
              // Game Details View
              <div className="flex-1 overflow-auto p-6">
                 <button 
                   onClick={handleBackToSearch}
                   className="mb-6 px-4 py-2 bg-sidebar hover:bg-sidebar-active rounded-md flex items-center gap-2 transition-colors duration-200"
                 >
                   <FiArrowLeft />
                   <span>Back to Search</span>
                 </button>
 
                 <div className="bg-surface rounded-lg p-6 border border-border">
                   <h2 className="text-2xl font-bold text-white mb-6">
                     {selectedGame.name || selectedGame.game_name}
                   </h2>
                   <div className="flex gap-6">
                     <div className="w-96 h-48 bg-sidebar rounded-lg overflow-hidden flex items-center justify-center relative">
                       {headerImageUrl ? (
                         <>
                           {!isLoaded && (
                             <div className="absolute inset-0 bg-sidebar animate-pulse flex items-center justify-center">
                               <IoGameController className="text-primary text-6xl animate-bounce" />
                             </div>
                           )}
                           <img 
                             src={headerImageUrl}
                             alt={selectedGame.name || selectedGame.game_name}
                             className={`w-full h-full object-cover transition-opacity duration-300 ${
                               isLoaded ? 'opacity-100' : 'opacity-0'
                             }`}
                           />
                         </>
                       ) : (
                         <div className="w-full h-full flex items-center justify-center">
                           <IoGameController className="text-primary text-6xl" />
                         </div>
                       )}
                     </div>
                     <div className="flex-1">
                       <div className="grid grid-cols-2 gap-x-4 gap-y-2">
                         <div>
                           <p className="text-gray-400">AppID</p>
                           <p className="text-white">{selectedGame.steam_appid || selectedGame.app_id}</p>
                         </div>
                         <div>
                           <p className="text-gray-400">Publisher</p>
                           <p className="text-white">{Array.isArray(selectedGame.publishers) && selectedGame.publishers.length > 0 ? selectedGame.publishers[0] : 'Unknown'}</p>
                         </div>
                         <div>
                           <p className="text-gray-400">Developer</p>
                           <p className="text-white">{Array.isArray(selectedGame.developers) && selectedGame.developers.length > 0 ? selectedGame.developers[0] : 'Unknown'}</p>
                         </div>
                         <div>
                           <p className="text-gray-400">Release Date</p>
                           <p className="text-white">{selectedGame.release_date?.date || 'Unknown'}</p>
                         </div>
                       </div>
                     </div>
                   </div>
                   {selectedGame.short_description && (
                     <div className="mt-6">
                       <h3 className="text-xl font-bold mb-2">About the Game</h3>
                       <p className="text-white leading-relaxed" dangerouslySetInnerHTML={{ __html: selectedGame.short_description }}></p>
                     </div>
                   )}
                   <div className="mt-6 flex gap-4">
                     <a 
                       href={`https://store.steampowered.com/app/${selectedGame.steam_appid || selectedGame.app_id}`}
                       target="_blank"
                       rel="noopener noreferrer"
                       className="inline-flex items-center gap-2 px-4 py-2 bg-[#171a21] hover:bg-[#1b2838] rounded-lg transition-colors duration-200"
                     >
                       <IoGameController className="w-5 h-5" />
                       <span>View on Steam Store</span>
                     </a>
                     <button
                       onClick={() => handleDownload(selectedGame)}
                       disabled={isDownloading}
                       className={`inline-flex items-center gap-2 px-4 py-2 rounded-lg transition-colors duration-200 ${
                         downloadStatus === 'idle' ? 'bg-primary hover:bg-highlight' :
                         downloadStatus === 'downloading' ? 'bg-yellow-600' :
                         downloadStatus === 'success' ? 'bg-green-600' :
                         'bg-red-600'
                       }`}
                     >
                       {downloadStatus === 'idle' && ( <><FiDownload /><span>Download</span></> )}
                       {downloadStatus === 'downloading' && ( <><span>Downloading...</span></> )}
                       {downloadStatus === 'success' && ( <><span>Downloaded</span></> )}
                       {downloadStatus === 'error' && ( <><span>Failed</span></> )}
                     </button>
                   </div>
                 </div>
              </div>
            ) : (
              // Search and Grid View
              <>
                <div className="p-6">
                  <div className="flex justify-between items-center mb-6">
                     <h2 className="text-2xl font-semibold">Explore Games</h2>
                     <div className="relative">
                       <input
                         type="text"
                         value={searchQuery}
                         onChange={(e) => setSearchQuery(e.target.value)}
                         onKeyDown={(e) => {
                           if (e.key === 'Enter') {
                             handleSearch(extractAppIdFromUrl(searchQuery), 1, true);
                           }
                         }}
                         placeholder="Search for games or AppID..."
                         className="w-64 pl-10 pr-4 py-2 bg-input border border-border rounded-full focus:outline-none focus:ring-2 focus:ring-primary"
                       />
                       <FiSearch className="absolute left-3 top-1/2 transform -translate-y-1/2 text-gray-400" />
                     </div>
                  </div>
                </div>

                {isLoading && <div className="text-center py-10">Loading...</div>}
                {error && <div className="text-center py-10 text-red-500">{error}</div>}
                
                {!isLoading && !error && (
                  <div className="flex-1 p-6 pt-0 overflow-y-auto">
                    <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-4">
                      {searchResults.games.map((game, index) => (
                        <GameCard key={`${game.app_id}-${index}`} game={game} onShowDetails={handleShowDetails} onDownload={handleDownload} isDownloading={isDownloading} />
                      ))}
                    </div>
                  </div>
                )}

                {!isLoading && searchResults.total_pages > 1 && (
                  <div className="flex justify-center items-center p-6 border-t border-border">
                    <button onClick={() => handlePageChange(currentPage - 1)} disabled={currentPage === 1} className="p-2 rounded-full hover:bg-gray-700 disabled:opacity-50 disabled:cursor-not-allowed">
                      <BsChevronLeft />
                    </button>
                    <span className="mx-4">
                      Page {searchResults.page} of {searchResults.total_pages}
                    </span>
                    <button onClick={() => handlePageChange(currentPage + 1)} disabled={currentPage === searchResults.total_pages} className="p-2 rounded-full hover:bg-gray-700 disabled:opacity-50 disabled:cursor-not-allowed">
                      <BsChevronRight />
                    </button>
                  </div>
                )}
              </>
            )}
          </div>
        )}

        {activeTab === 'sharing' && (
          <div className="flex-1 p-6 flex flex-col overflow-hidden">
            <div className="flex justify-between items-center mb-6">
              <h2 className="text-2xl font-semibold">Account Sharing</h2>
              <div className="flex items-center gap-4">
                <div className="relative">
                  <input
                    type="text"
                    value={accountSearchQuery}
                    onChange={(e) => {
                        setAccountSearchQuery(e.target.value);
                        setAccountCurrentPage(1); // Reset to first page on new search
                    }}
                    placeholder="Search accounts..."
                    className="w-64 pl-10 pr-4 py-2 bg-input border border-border rounded-full focus:outline-none focus:ring-2 focus:ring-primary"
                  />
                  <FiSearch className="absolute left-3 top-1/2 transform -translate-y-1/2 text-gray-400" />
                </div>
                <button
                  onClick={() => setIsManagingAccounts(true)}
                  className="flex items-center gap-2 px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 transition-colors"
                >
                  <FiEdit />
                  <span>Manage Accounts</span>
                </button>
              </div>
            </div>
            <div className="flex-1 overflow-y-auto">
              <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-4">
                {currentAccounts.map((account, index) => (
                  <div
                    key={index}
                    className="bg-card rounded-lg overflow-hidden shadow-lg cursor-pointer transform hover:-translate-y-1 transition-transform duration-300"
                    onClick={() => handleSwitchAccount(account)}
                  >
                    <img src={account.imageUrl} alt={account.gameName} className="w-full h-32 object-cover" />
                    <div className="p-4">
                      <h3 className="font-bold text-lg truncate">{account.displayName}</h3>
                      <p className="text-gray-400 text-sm mb-1">{account.gameName}</p>
                      {account.drm && account.drm.trim() !== '' && (
                        <div className="mt-2 flex items-center gap-2 text-yellow-500 text-xs font-semibold">
                          <FiAlertTriangle />
                          <span className="truncate">{account.drm}</span>
                        </div>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            </div>
            {totalAccountPages > 1 && (
                <div className="flex justify-center items-center mt-6 pt-4 border-t border-border">
                    <button onClick={() => handleAccountPageChange(accountCurrentPage - 1)} disabled={accountCurrentPage === 1} className="p-2 rounded-full hover:bg-gray-700 disabled:opacity-50 disabled:cursor-not-allowed">
                        <BsChevronLeft />
                    </button>
                    <span className="mx-4">
                        Page {accountCurrentPage} of {totalAccountPages}
                    </span>
                    <button onClick={() => handleAccountPageChange(accountCurrentPage + 1)} disabled={accountCurrentPage === totalAccountPages} className="p-2 rounded-full hover:bg-gray-700 disabled:opacity-50 disabled:cursor-not-allowed">
                        <BsChevronRight />
                    </button>
                </div>
            )}
          </div>
        )}

        {activeTab === 'library' && (
          <MyLibrary onShowDetails={handleShowDetails} showNotification={showNotification} />
        )}

        {activeTab === 'notes' && (
          <NotesManager showNotification={showNotification} />
        )}

        {activeTab === 'settings' && (
          <SettingsPanel showNotification={showNotification} />
        )}
      </main>

      {isManagingAccounts && (
        <AccountManager
          accounts={accounts}
          onClose={() => setIsManagingAccounts(false)}
          onSave={handleSaveAccounts}
          showNotification={showNotification}
        />
      )}

      {/* Notification Popup */}
      {notification && (
        <div className="fixed bottom-5 right-5 z-50 animate-slide-up-fade-in">
          <div className={`flex items-center gap-3 max-w-sm px-4 py-3 rounded-lg shadow-2xl border-l-4 ${
            notification.type === 'error' ? 'bg-red-500 border-red-700' :
            notification.type === 'success' ? 'bg-green-500 border-green-700' :
            'bg-blue-500 border-blue-700'
          }`}>
            <div className="flex-shrink-0 text-white">
              {notification.type === 'error' && (
                <FiAlertTriangle size={24} />
              )}
              {notification.type === 'success' && (
                 <svg className="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
              )}
              {notification.type === 'info' && (
                <svg className="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
              )}
            </div>
            <div className="flex-1">
              <p className="text-white font-semibold">
                {notification.message}
              </p>
            </div>
            <button 
              onClick={closeNotification}
              className="ml-4 -mr-1 p-1 rounded-full text-white/70 hover:text-white hover:bg-black/20 transition-all"
            >
              <FiX size={18} />
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

export default App; 