import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { FiX, FiLoader } from 'react-icons/fi';

const DlcManager = ({ game, onClose, showNotification }) => {
  const [dlcs, setDlcs] = useState([]); // Details for the current page
  const [allDlcAppIds, setAllDlcAppIds] = useState([]); // All available DLC AppIDs
  const [installedDlcs, setInstalledDlcs] = useState(new Set());
  const [selectedDlcs, setSelectedDlcs] = useState(new Set());
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState(null);
  const [currentPage, setCurrentPage] = useState(1);
  const dlcsPerPage = 10;

  // Step 1: Fetch the full list of DLC AppIDs and installed DLCs once.
  useEffect(() => {
    const fetchInitialDlcData = async () => {
      setIsLoading(true);
      setError(null);
      try {
        // Get all DLC AppIDs for the main game
        const gameDetails = await invoke('get_game_details', { appId: game.app_id });
        const allIds = (gameDetails.dlc || []).map(String);

        if (allIds.length === 0) {
          setError('This game has no available DLCs.');
          setIsLoading(false);
          return;
        }
        setAllDlcAppIds(allIds);

        // Get currently installed DLCs from the LUA file
        const installed = await invoke('get_dlcs_in_lua', { appId: game.app_id });
        const installedSet = new Set(installed);
        setInstalledDlcs(installedSet);
        setSelectedDlcs(installedSet);
        
      } catch (err) {
        setError(`Failed to load initial DLC list: ${err.toString()}`);
        setAllDlcAppIds([]);
      } finally {
        // Loading is handled by the next effect, which fetches the first page
      }
    };

    fetchInitialDlcData();
  }, [game.app_id]);

  // Step 2: Fetch details for the current page whenever page or the full list changes.
  useEffect(() => {
    if (allDlcAppIds.length === 0) {
      // Don't start loading if there are no IDs to fetch.
      // The "no DLCs" message will be shown from the first effect.
      return;
    }

    const fetchDlcPageDetails = async () => {
      setIsLoading(true);
      setDlcs([]); // Clear previous page's content

      const startIndex = (currentPage - 1) * dlcsPerPage;
      const endIndex = startIndex + dlcsPerPage;
      const pageAppIds = allDlcAppIds.slice(startIndex, endIndex);

      if (pageAppIds.length === 0) {
        setIsLoading(false);
        return;
      }

      try {
        const dlcDetails = await invoke('get_batch_game_details', { appIds: pageAppIds });
        setDlcs(dlcDetails);
      } catch (err) {
        setError(`Failed to load DLC details for page ${currentPage}: ${err.toString()}`);
      } finally {
        setIsLoading(false);
      }
    };

    fetchDlcPageDetails();
  }, [currentPage, allDlcAppIds]);

  const handleAddDlc = (dlcId) => {
    setSelectedDlcs(prev => {
      const newSet = new Set(prev);
      newSet.add(dlcId.toString());
      return newSet;
    });
  };

  const handleRemoveDlc = (dlcId) => {
    setSelectedDlcs(prev => {
      const newSet = new Set(prev);
      newSet.delete(dlcId.toString());
      return newSet;
    });
  };

  const handleSaveChanges = async () => {
    setIsSaving(true);
    
    try {
      const result = await invoke('sync_dlcs_in_lua', {
        mainAppId: game.app_id,
        dlcIdsToSet: Array.from(selectedDlcs),
      });
      showNotification(result, 'success');
      onClose();
    } catch (err) {
      showNotification(`Error saving DLCs: ${err.toString()}`, 'error');
    } finally {
      setIsSaving(false);
    }
  };

  const totalPages = Math.ceil(allDlcAppIds.length / dlcsPerPage);

  const handlePageChange = (newPage) => {
    if (newPage >= 1 && newPage <= totalPages) {
      setCurrentPage(newPage);
    }
  };

  return (
    <div className="fixed inset-0 bg-black bg-opacity-75 flex items-center justify-center z-50 animate-fade-in-fast">
      <div className="bg-surface rounded-lg w-full max-w-2xl h-full max-h-[80vh] flex flex-col border border-border">
        {/* Header */}
        <div className="p-4 border-b border-border flex justify-between items-center">
          <h2 className="text-xl font-bold">Manage DLCs for {game.name}</h2>
          <button onClick={onClose} className="text-gray-400 hover:text-white">
            <FiX size={24} />
          </button>
        </div>

        {/* Content */}
        <div className="p-6 flex-1 overflow-y-auto">
          {isLoading && (
            <div className="flex items-center justify-center h-full">
              <FiLoader className="animate-spin text-primary" size={48} />
              <p className="ml-4 text-lg">Loading DLC Information...</p>
            </div>
          )}

          {error && !isLoading && (
            <div className="text-center text-red-400 p-8">{error}</div>
          )}

          {!isLoading && !error && allDlcAppIds.length > 0 && (
            <div className="space-y-3">
              {dlcs.map(dlc => {
                const dlcIdStr = dlc.steam_appid.toString();
                const isSelected = selectedDlcs.has(dlcIdStr);

                return (
                  <div 
                    key={dlc.steam_appid} 
                    className="bg-sidebar p-3 rounded-md flex items-center justify-between gap-4"
                  >
                    <div>
                      <p className="font-medium text-white">{dlc.name}</p>
                      <p className="text-xs text-gray-400">AppID: {dlc.steam_appid}</p>
                    </div>
                    <div className="flex gap-2">
                      <button
                        onClick={() => handleAddDlc(dlc.steam_appid)}
                        disabled={isSelected}
                        className="px-4 py-1.5 rounded-md text-sm font-semibold transition-colors flex-shrink-0 bg-primary hover:bg-highlight text-white disabled:bg-gray-600 disabled:cursor-not-allowed"
                      >
                        Add
                      </button>
                      <button
                        onClick={() => handleRemoveDlc(dlc.steam_appid)}
                        disabled={!isSelected}
                        className="px-4 py-1.5 rounded-md text-sm font-semibold transition-colors flex-shrink-0 bg-red-600 hover:bg-red-700 text-white disabled:bg-gray-600 disabled:cursor-not-allowed"
                      >
                        Remove
                      </button>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>

        {/* Pagination Controls */}
        {totalPages > 1 && !error && (
            <div className="p-2 border-t border-border flex justify-center items-center gap-2">
                <button
                    onClick={() => handlePageChange(currentPage - 1)}
                    disabled={currentPage === 1 || isLoading}
                    className="px-4 py-2 bg-primary hover:bg-highlight rounded-md transition-colors disabled:bg-gray-600 disabled:cursor-not-allowed"
                >
                    Previous
                </button>
                <span className="text-gray-300">
                    Page {currentPage} of {totalPages}
                </span>
                <button
                    onClick={() => handlePageChange(currentPage + 1)}
                    disabled={currentPage === totalPages || isLoading}
                    className="px-4 py-2 bg-primary hover:bg-highlight rounded-md transition-colors disabled:bg-gray-600 disabled:cursor-not-allowed"
                >
                    Next
                </button>
            </div>
        )}

        {/* Footer */}
        <div className="p-4 border-t border-border flex justify-end gap-4">
          <button 
            onClick={onClose} 
            className="px-4 py-2 bg-gray-600 hover:bg-gray-700 rounded-md transition-colors"
          >
            Cancel
          </button>
          <button 
            onClick={handleSaveChanges} 
            disabled={isSaving || isLoading}
            className="px-4 py-2 bg-primary hover:bg-highlight rounded-md transition-colors disabled:bg-gray-500 flex items-center gap-2"
          >
            {isSaving && <FiLoader className="animate-spin" />}
            {isSaving ? 'Saving...' : 'Save Changes'}
          </button>
        </div>
      </div>
    </div>
  );
};

export default DlcManager; 