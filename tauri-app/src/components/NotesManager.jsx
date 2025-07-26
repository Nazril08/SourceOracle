import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { FiPlus, FiSave, FiEdit2, FiTrash2, FiX } from 'react-icons/fi';

const NotesManager = ({ showNotification }) => {
  const [notes, setNotes] = useState([]);
  const [isEditing, setIsEditing] = useState(null); // null, 'new', or note.id
  const [editForm, setEditForm] = useState({ id: '', title: '', content: '' });

  useEffect(() => {
    loadNotes();
  }, []);

  const loadNotes = async () => {
    try {
      const fetchedNotes = await invoke('get_notes');
      setNotes(fetchedNotes);
    } catch (err) {
      showNotification(`Error loading notes: ${err}`, 'error');
    }
  };

  const handleAddNew = () => {
    setEditForm({ id: '', title: '', content: '' });
    setIsEditing('new');
  };

  const handleEdit = (note) => {
    setEditForm({ ...note });
    setIsEditing(note.id);
  };

  const handleDelete = async (id) => {
    try {
      const updatedNotes = await invoke('delete_note', { id });
      setNotes(updatedNotes);
      showNotification('Note deleted successfully', 'success');
    } catch (err) {
      showNotification(`Error deleting note: ${err}`, 'error');
    }
  };

  const handleFormChange = (e) => {
    setEditForm({ ...editForm, [e.target.name]: e.target.value });
  };

  const handleSave = async () => {
    if (!editForm.title.trim()) {
      showNotification('Title cannot be empty', 'error');
      return;
    }

    try {
      if (isEditing === 'new') {
        const updatedNotes = await invoke('add_note', { title: editForm.title, content: editForm.content });
        setNotes(updatedNotes);
        showNotification('Note added successfully', 'success');
      } else {
        const updatedNotes = await invoke('update_note', { note: editForm });
        setNotes(updatedNotes);
        showNotification('Note updated successfully', 'success');
      }
      setIsEditing(null);
      setEditForm({ id: '', title: '', content: '' });
    } catch (err) {
      showNotification(`Error saving note: ${err}`, 'error');
    }
  };
  
  const handleCancel = () => {
    setIsEditing(null);
    setEditForm({ id: '', title: '', content: '' });
  };

  const renderForm = () => (
    <div className="fixed inset-0 bg-black bg-opacity-75 flex justify-center items-center z-50">
        <div className="bg-sidebar w-full max-w-2xl rounded-lg shadow-lg flex flex-col p-6 border border-border">
            <div className="flex justify-between items-center mb-4">
                <h2 className="text-2xl font-bold">{isEditing === 'new' ? 'Add New Note' : 'Edit Note'}</h2>
                <button onClick={handleCancel} className="p-2 rounded-full hover:bg-sidebar-active">
                    <FiX size={24} />
                </button>
            </div>
            <div className="flex flex-col gap-4">
                <input
                    type="text"
                    name="title"
                    value={editForm.title}
                    onChange={handleFormChange}
                    placeholder="Note Title"
                    className="bg-input p-3 rounded border border-border text-lg"
                />
                <textarea
                    name="content"
                    value={editForm.content}
                    onChange={handleFormChange}
                    placeholder="Note content..."
                    className="bg-input p-3 rounded border border-border h-64 resize-none"
                    rows="10"
                ></textarea>
            </div>
            <div className="mt-6 flex justify-end gap-4">
                <button onClick={handleCancel} className="px-4 py-2 bg-gray-600 rounded hover:bg-gray-700">Cancel</button>
                <button onClick={handleSave} className="px-4 py-2 bg-blue-600 rounded hover:bg-blue-700 flex items-center gap-2">
                    <FiSave /> Save
                </button>
            </div>
        </div>
    </div>
  );

  return (
    <div className="p-8 text-white h-full overflow-y-auto">
      {isEditing && renderForm()}
      <div className="flex justify-between items-center mb-6">
        <h1 className="text-3xl font-bold">Notes</h1>
        <button onClick={handleAddNew} className="flex items-center gap-2 px-4 py-2 bg-green-600 text-white rounded-md hover:bg-green-700 transition-colors">
          <FiPlus />
          <span>Add New Note</span>
        </button>
      </div>
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
        {notes.map(note => (
          <div key={note.id} className="bg-surface rounded-lg p-4 flex flex-col justify-between border border-border hover:border-blue-500 transition-all">
            <div>
              <h3 className="text-xl font-semibold mb-2">{note.title}</h3>
              <p className="text-gray-400 whitespace-pre-wrap">{note.content.substring(0, 150)}{note.content.length > 150 ? '...' : ''}</p>
            </div>
            <div className="flex justify-end gap-2 mt-4">
              <button onClick={() => handleEdit(note)} className="p-2 bg-blue-600 rounded hover:bg-blue-700"><FiEdit2 /></button>
              <button onClick={() => handleDelete(note.id)} className="p-2 bg-red-600 rounded hover:bg-red-700"><FiTrash2 /></button>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};

export default NotesManager; 