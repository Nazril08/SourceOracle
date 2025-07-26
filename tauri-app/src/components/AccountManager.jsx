import React, { useState } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { ask, save, open } from '@tauri-apps/api/dialog';
import { writeTextFile, readTextFile } from '@tauri-apps/api/fs';
import { FiX, FiPlus, FiTrash2, FiSave, FiUpload, FiDownload } from 'react-icons/fi';

const AccountManager = ({ accounts, onClose, onSave, showNotification }) => {
  const [localAccounts, setLocalAccounts] = useState([...accounts]);
  const [isEditing, setIsEditing] = useState(null); // index of account being edited, or 'new'
  const [editForm, setEditForm] = useState(null);

  const handleAddNew = () => {
    setEditForm({
      gameName: '',
      displayName: '',
      steamUsername: '',
      steamPassword: '',
      imageUrl: '',
      drm: '', // Default to an empty string, which represents "None"
    });
    setIsEditing('new');
  };

  const handleEdit = (account, index) => {
    setEditForm({ ...account });
    setIsEditing(index);
  };

  const handleDelete = async (index) => {
    const shouldDelete = await ask('Are you sure you want to delete this account?', {
      title: 'Confirm Deletion',
      type: 'warning',
    });
    
    if (!shouldDelete) {
      return;
    }
    
    try {
      const updatedAccounts = await invoke('delete_account', { index });
      setLocalAccounts(updatedAccounts);
      onSave(updatedAccounts);
      showNotification('Account deleted successfully', 'success');
    } catch (err) {
      showNotification(`Error deleting account: ${err}`, 'error');
    }
  };

  const handleFormChange = (e) => {
    setEditForm({
      ...editForm,
      [e.target.name]: e.target.value,
    });
  };

  const handleSaveEdit = async () => {
    // If DRM is 'None', save it as an empty string
    const accountToSave = {
      ...editForm,
      drm: editForm.drm === 'None' ? '' : editForm.drm,
    };

    if (isEditing === 'new') {
      // Add new account
      try {
        const updatedAccounts = await invoke('add_account', { account: accountToSave });
        setLocalAccounts(updatedAccounts);
        onSave(updatedAccounts);
        showNotification('Account added successfully', 'success');
      } catch (err) {
        showNotification(`Error adding account: ${err}`, 'error');
      }
    } else {
      // Update existing account
      try {
        const updatedAccounts = await invoke('update_account', { index: isEditing, account: accountToSave });
        setLocalAccounts(updatedAccounts);
        onSave(updatedAccounts);
        showNotification('Account updated successfully', 'success');
      } catch (err) {
        showNotification(`Error updating account: ${err}`, 'error');
      }
    }
    setIsEditing(null);
    setEditForm(null);
  };

  const handleClose = () => {
    onClose();
  };

  const handleExport = async () => {
    try {
      const filePath = await save({
        filters: [{
          name: 'JSON',
          extensions: ['json']
        }],
        defaultPath: 'accounts-backup.json'
      });

      if (filePath) {
        const accountsJson = JSON.stringify(localAccounts, null, 2);
        await writeTextFile(filePath, accountsJson);
        showNotification('Accounts exported successfully!', 'success');
      }
    } catch (err) {
      showNotification(`Error exporting accounts: ${err}`, 'error');
    }
  };

  const handleImport = async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [{
          name: 'JSON',
          extensions: ['json']
        }]
      });

      if (selected && !Array.isArray(selected)) {
        const fileContents = await readTextFile(selected);
        const importedAccounts = JSON.parse(fileContents);
        
        const confirmed = await ask('Are you sure you want to replace all current accounts with the imported data?', {
          title: 'Confirm Import',
          type: 'warning'
        });

        if (confirmed) {
          const updatedAccounts = await invoke('import_accounts', { accounts: importedAccounts });
          setLocalAccounts(updatedAccounts);
          onSave(updatedAccounts);
          showNotification('Accounts imported successfully!', 'success');
        }
      }
    } catch (err) {
      showNotification(`Error importing accounts: ${err}`, 'error');
    }
  };

  return (
    <div className="fixed inset-0 bg-black bg-opacity-75 flex justify-center items-center z-50">
      <div className="bg-sidebar w-full max-w-4xl h-[80vh] rounded-lg shadow-lg flex flex-col p-6 border border-border">
        <div className="flex justify-between items-center mb-4">
          <h2 className="text-2xl font-bold">Manage Accounts</h2>
          <button onClick={handleClose} className="p-2 rounded-full hover:bg-sidebar-active">
            <FiX size={24} />
          </button>
        </div>

        <div className="flex-1 overflow-y-auto">
          {isEditing !== null ? (
            // Form View
            <div className="bg-surface p-6 rounded-lg">
              <h3 className="text-xl font-semibold mb-4">{isEditing === 'new' ? 'Add New Account' : 'Edit Account'}</h3>
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                <input type="text" name="displayName" value={editForm.displayName} onChange={handleFormChange} placeholder="Display Name (e.g., Hogwarts Legacy 1)" className="bg-input p-2 rounded border border-border" />
                <input type="text" name="gameName" value={editForm.gameName} onChange={handleFormChange} placeholder="Game Name (e.g., Hogwarts Legacy)" className="bg-input p-2 rounded border border-border" />
                <input type="text" name="steamUsername" value={editForm.steamUsername} onChange={handleFormChange} placeholder="Steam Username" className="bg-input p-2 rounded border border-border" />
                <input type="password" name="steamPassword" value={editForm.steamPassword} onChange={handleFormChange} placeholder="Steam Password" className="bg-input p-2 rounded border border-border" />
                {/* TODO: Anda bisa menambahkan opsi DRM baru di sini */}
                <select name="drm" value={editForm.drm || ''} onChange={handleFormChange} className="bg-input p-2 rounded border border-border">
                  <option value="">None</option>
                  <option value="Denuvo">Denuvo</option>
                  <option value="Rockstar Launcher">Rockstar Launcher</option>
                  <option value="Ubisoft Connect">Ubisoft Connect</option>
                  <option value="EA App">EA App</option>
                  <option value="Battle.net DRM">Battle.net DRM</option>
                  <option value="Xbox DRM">Xbox DRM</option>
                  <option value="Paradox Launcher">Paradox Launcher</option>
                </select>
                <input type="text" name="imageUrl" value={editForm.imageUrl} onChange={handleFormChange} placeholder="Image URL (e.g., from Steam header)" className="bg-input p-2 rounded border border-border" />
              </div>
              <div className="mt-6 flex justify-end gap-4">
                <button onClick={() => setIsEditing(null)} className="px-4 py-2 bg-gray-600 rounded hover:bg-gray-700">Cancel</button>
                <button onClick={handleSaveEdit} className="px-4 py-2 bg-blue-600 rounded hover:bg-blue-700 flex items-center gap-2"><FiSave /> Save</button>
              </div>
            </div>
          ) : (
            // List View
            <>
              <div className="flex justify-end mb-4 gap-2">
                <button onClick={handleImport} className="flex items-center gap-2 px-4 py-2 bg-purple-600 text-white rounded-md hover:bg-purple-700 transition-colors">
                  <FiUpload />
                  <span>Import</span>
                </button>
                <button onClick={handleExport} className="flex items-center gap-2 px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 transition-colors">
                  <FiDownload />
                  <span>Export</span>
                </button>
                <button onClick={handleAddNew} className="flex items-center gap-2 px-4 py-2 bg-green-600 text-white rounded-md hover:bg-green-700 transition-colors">
                  <FiPlus />
                  <span>Add New Account</span>
                </button>
              </div>
              <table className="w-full text-left">
                <thead className="border-b border-border">
                  <tr>
                    <th className="p-2">Display Name</th>
                    <th className="p-2">Game</th>
                    <th className="p-2">DRM</th>
                    <th className="p-2">Username</th>
                    <th className="p-2 text-right">Actions</th>
                  </tr>
                </thead>
                <tbody>
                  {localAccounts.map((account, index) => (
                    <tr key={index} className="border-b border-sidebar-active">
                      <td className="p-2">{account.displayName}</td>
                      <td className="p-2">{account.gameName}</td>
                      <td className="p-2">{account.drm || 'N/A'}</td>
                      <td className="p-2">{account.steamUsername}</td>
                      <td className="p-2 flex justify-end gap-2">
                        <button onClick={() => handleEdit(account, index)} className="p-2 bg-blue-600 rounded hover:bg-blue-700">Edit</button>
                        <button onClick={() => handleDelete(index)} className="p-2 bg-red-600 rounded hover:bg-red-700"><FiTrash2 /></button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </>
          )}
        </div>
      </div>
    </div>
  );
};

export default AccountManager; 