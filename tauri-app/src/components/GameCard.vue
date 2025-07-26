<template>
  <div class="game-card" @click="selectGame">
    <div class="game-image">
      <img 
        v-if="game.icon_url" 
        :src="game.icon_url" 
        :alt="game.game_name"
        @error="handleImageError"
      />
      <div v-else class="placeholder-image">
        <i class="fas fa-gamepad"></i>
      </div>
    </div>
    <div class="game-info">
      <h3 class="game-title">{{ truncatedTitle }}</h3>
      <p class="game-id">AppID: {{ game.app_id }}</p>
    </div>
    <div class="game-actions">
      <button class="detail-button" @click.stop="showGameDetails">
        Detail Game
      </button>
    </div>
  </div>
</template>

<script>
import { invoke } from '@tauri-apps/api/tauri';

export default {
  name: 'GameCard',
  props: {
    game: {
      type: Object,
      required: true
    }
  },
  computed: {
    truncatedTitle() {
      // Truncate title if too long
      const maxLength = 30;
      if (this.game.game_name.length > maxLength) {
        return this.game.game_name.substring(0, maxLength) + '...';
      }
      return this.game.game_name;
    }
  },
  methods: {
    selectGame() {
      this.$emit('select', this.game);
    },
    handleImageError(e) {
      // Replace broken image with placeholder
      e.target.src = 'https://via.placeholder.com/460x215/2c2c2e/ffffff?text=No+Image';
    },
    async showGameDetails() {
      try {
        // Fetch game details from Steam API
        const gameDetails = await invoke('get_game_details', { 
          appId: this.game.app_id 
        });
        
        // Emit event with game details to show the details page
        this.$emit('show-details', gameDetails);
      } catch (error) {
        console.error('Error loading game details:', error);
        alert(`Failed to load game details: ${error}`);
      }
    }
  }
};
</script>

<style scoped>
.game-card {
  background-color: #2c2c2e;
  border-radius: 10px;
  overflow: hidden;
  transition: transform 0.3s, box-shadow 0.3s;
  cursor: pointer;
  display: flex;
  flex-direction: column;
}

.game-card:hover {
  transform: translateY(-5px);
  box-shadow: 0 10px 20px rgba(0, 0, 0, 0.3);
}

.game-image {
  height: 145px;
  overflow: hidden;
  position: relative;
}

.game-image img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  transition: transform 0.3s;
}

.game-card:hover .game-image img {
  transform: scale(1.05);
}

.placeholder-image {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: #3a3a3c;
  color: #8e8e93;
  font-size: 40px;
}

.game-info {
  padding: 12px;
  flex-grow: 1;
}

.game-title {
  margin: 0 0 8px 0;
  font-size: 16px;
  font-weight: 600;
  color: #ffffff;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.game-id {
  margin: 0;
  font-size: 12px;
  color: #8e8e93;
}

.game-actions {
  padding: 0 12px 12px;
}

.detail-button {
  width: 100%;
  padding: 8px 0;
  background-color: #6366f1;
  color: white;
  border: none;
  border-radius: 6px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: background-color 0.3s;
}

.detail-button:hover {
  background-color: #4f46e5;
}
</style> 