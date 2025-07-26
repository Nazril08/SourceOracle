import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { open } from '@tauri-apps/api/dialog';
import { FiFolder, FiSave, FiAlertCircle } from 'react-icons/fi';

const SettingsPanel = ({ showNotification }) => {
  const [downloadDir, setDownloadDir] = useState('');
  const [isLoading, setIsLoading] = useState(true);

  // Load settings on component mount
  useEffect(() => {
    const loadSettings = async () => {
      try {
        const settings = await invoke('load_settings');
        setDownloadDir(settings.download_directory || '');
      } catch (error) {
        showNotification(`Error loading settings: ${error}`, 'error');
      } finally {
        setIsLoading(false);
      }
    };
    loadSettings();
  }, []);

  const handleBrowse = async () => {
    try {
      const selectedPath = await open({
        directory: true,
        multiple: false,
        defaultPath: downloadDir || undefined,
      });
      if (typeof selectedPath === 'string') {
        setDownloadDir(selectedPath);
      }
    } catch (error) {
      showNotification(`Error opening folder dialog: ${error}`, 'error');
    }
  };

  const handleSave = async () => {
    try {
      await invoke('save_settings', {
        settings: { download_directory: downloadDir },
      });
      showNotification('Settings saved successfully!', 'success');
    } catch (error) {
      showNotification(`Error saving settings: ${error}`, 'error');
    }
  };

  if (isLoading) {
    return (
      <div className="flex-1 p-6 text-center">
        <p>Loading settings...</p>
      </div>
    );
  }

  return (
    <div className="flex-1 p-6 bg-background text-white">
      <h1 className="text-3xl font-bold mb-6">Settings</h1>

      <div className="bg-surface p-6 rounded-lg border border-border">
        <h2 className="text-xl font-bold mb-4">Download Location</h2>
        <p className="text-gray-400 mb-2">
          Select the folder where game files will be downloaded.
        </p>

        <div className="bg-sidebar p-3 rounded-md mb-4">
          <p className="text-sm text-gray-400">Current Directory:</p>
          <p className="font-mono text-white">{downloadDir || 'Not set'}</p>
        </div>

        <div className="flex gap-4">
          <button
            onClick={handleBrowse}
            className="flex items-center gap-2 px-4 py-2 bg-primary hover:bg-highlight rounded-md transition-colors"
          >
            <FiFolder />
            <span>Browse</span>
          </button>
          <button
            onClick={handleSave}
            className="flex items-center gap-2 px-4 py-2 bg-green-600 hover:bg-green-700 rounded-md transition-colors"
          >
            <FiSave />
            <span>Save</span>
          </button>
        </div>
        
        {!downloadDir && (
          <div className="mt-4 flex items-center text-yellow-400">
            <FiAlertCircle className="mr-2" />
            <p>Please select a download directory.</p>
          </div>
        )}
      </div>
    </div>
  );
};

export default SettingsPanel; 