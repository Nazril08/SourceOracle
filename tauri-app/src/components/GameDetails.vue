<template>
  <div class="game-details">
    <div v-if="loading" class="loading">
      <div class="spinner"></div>
      <p>Loading game details...</p>
    </div>
    
    <div v-else-if="error" class="error">
      <p>{{ error }}</p>
      <button @click="loadGameDetails" class="retry-button">Retry</button>
    </div>
    
    <div v-else-if="game" class="game-info">
      <div class="game-header">
        <img 
          v-if="game.header_image" 
          :src="game.header_image" 
          :alt="game.name"
          @error="handleImageError"
          class="game-banner"
        />
        <div v-else class="placeholder-banner">
          <i class="fas fa-gamepad"></i>
        </div>
        
        <div class="game-title-container">
          <h2 class="game-title">{{ game.name || game.game_name }}</h2>
          <p class="game-id">AppID: {{ game.steam_appid || game.app_id }}</p>
        </div>
      </div>
      
      <div class="game-stats">
        <div class="stat">
          <div class="stat-label">Release Date</div>
          <div class="stat-value">{{ releaseDate }}</div>
        </div>
        
        <div class="stat">
          <div class="stat-label">Developer</div>
          <div class="stat-value">{{ developers }}</div>
        </div>
        
        <div class="stat">
          <div class="stat-label">Publisher</div>
          <div class="stat-value">{{ publishers }}</div>
        </div>
        
        <div class="stat" v-if="game.price_overview">
          <div class="stat-label">Price</div>
          <div class="stat-value">{{ formatPrice(game.price_overview) }}</div>
        </div>
      </div>
      
      <div class="game-description" v-if="game.short_description">
        <h3>About the Game</h3>
        <p>{{ game.short_description }}</p>
      </div>
      
      <div class="game-tags" v-if="game.categories && game.categories.length > 0">
        <h3>Categories</h3>
        <div class="tags-container">
          <span class="tag" v-for="category in game.categories" :key="category.id">
            {{ category.description }}
          </span>
        </div>
      </div>
      
      <div class="game-tags" v-if="game.genres && game.genres.length > 0">
        <h3>Genres</h3>
        <div class="tags-container">
          <span class="tag" v-for="genre in game.genres" :key="genre.id">
            {{ genre.description }}
          </span>
        </div>
      </div>
      
      <div class="game-screenshots" v-if="game.screenshots && game.screenshots.length > 0">
        <h3>Screenshots</h3>
        <div class="screenshots-container">
          <img 
            v-for="screenshot in game.screenshots.slice(0, 4)" 
            :key="screenshot.id"
            :src="screenshot.path_thumbnail"
            alt="Game Screenshot"
            class="screenshot"
            @error="handleImageError"
          />
        </div>
      </div>
      
      <div class="steam-link" v-if="hasSteamId">
        <a :href="steamStoreUrl" target="_blank" rel="noopener noreferrer" class="steam-button">
          <i class="fab fa-steam"></i> View on Steam Store
        </a>
      </div>
    </div>
    
    <div v-else class="no-game-selected">
      <i class="fas fa-gamepad icon"></i>
      <p>No game details available</p>
    </div>
  </div>
</template>

<script>
import { invoke } from '@tauri-apps/api/tauri';

export default {
  name: 'GameDetails',
  props: {
    game: {
      type: Object,
      default: null
    },
    selectedGame: {
      type: Object,
      default: null
    }
  },
  data() {
    return {
      loading: false,
      error: null,
      steamDetails: null
    };
  },
  computed: {
    hasSteamId() {
      return this.game && (this.game.steam_appid || this.game.app_id);
    },
    steamAppId() {
      return this.game ? (this.game.steam_appid || this.game.app_id) : null;
    },
    steamStoreUrl() {
      if (!this.hasSteamId) return '#';
      return `https://store.steampowered.com/app/${this.steamAppId}`;
    },
    releaseDate() {
      if (!this.game || !this.game.release_date) return 'Unknown';
      return this.game.release_date.date || 'Unknown';
    },
    developers() {
      if (!this.game || !this.game.developers || !this.game.developers.length) return 'Unknown';
      return this.game.developers.join(', ');
    },
    publishers() {
      if (!this.game || !this.game.publishers || !this.game.publishers.length) return 'Unknown';
      return this.game.publishers.join(', ');
    }
  },
  watch: {
    selectedGame(newGame) {
      if (newGame) {
        this.loadGameDetails();
      }
    },
    game(newGame) {
      if (newGame && !this.steamDetails && newGame.app_id) {
        this.fetchSteamDetails(newGame.app_id);
      }
    }
  },
  methods: {
    async loadGameDetails() {
      if (!this.selectedGame) return;
      
      this.loading = true;
      this.error = null;
      
      try {
        const gameDetails = await invoke('get_game_details', { 
          appId: this.selectedGame.app_id 
        });
        
        this.steamDetails = gameDetails;
      } catch (error) {
        console.error('Error loading game details:', error);
        this.error = `Failed to load game details: ${error}`;
      } finally {
        this.loading = false;
      }
    },
    
    async fetchSteamDetails(appId) {
      if (!appId) return;
      
      this.loading = true;
      this.error = null;
      
      try {
        // This would be implemented in the Rust backend
        const steamDetails = await invoke('fetch_steam_details', { appId });
        this.steamDetails = steamDetails;
      } catch (error) {
        console.error('Error fetching Steam details:', error);
        // Don't set error, just log it
      } finally {
        this.loading = false;
      }
    },
    
    formatPrice(priceOverview) {
      if (!priceOverview) return 'Free';
      
      if (priceOverview.discount_percent > 0) {
        return `${priceOverview.final_formatted} (${priceOverview.discount_percent}% off)`;
      }
      
      return priceOverview.final_formatted;
    },
    
    handleImageError(e) {
      e.target.src = 'https://via.placeholder.com/460x215/2c2c2e/ffffff?text=No+Image';
    }
  },
  mounted() {
    if (this.selectedGame) {
      this.loadGameDetails();
    } else if (this.game && this.game.app_id) {
      this.fetchSteamDetails(this.game.app_id);
    }
  }
};
</script>

<style scoped>
.game-details {
  background-color: #1c1c1e;
  border-radius: 12px;
  padding: 20px;
  height: 100%;
  display: flex;
  flex-direction: column;
}

.loading, .error, .no-game-selected {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  text-align: center;
  color: #a1a1a6;
}

.spinner {
  border: 4px solid rgba(255, 255, 255, 0.1);
  border-radius: 50%;
  border-top: 4px solid #0071e3;
  width: 40px;
  height: 40px;
  animation: spin 1s linear infinite;
  margin-bottom: 20px;
}

@keyframes spin {
  0% { transform: rotate(0deg); }
  100% { transform: rotate(360deg); }
}

.icon {
  font-size: 48px;
  margin-bottom: 20px;
  color: #8e8e93;
}

.retry-button {
  margin-top: 15px;
  padding: 8px 16px;
  background-color: #0071e3;
  border: none;
  border-radius: 6px;
  color: white;
  cursor: pointer;
}

.game-info {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.game-header {
  position: relative;
  margin-bottom: 20px;
}

.game-banner {
  width: 100%;
  height: 200px;
  object-fit: cover;
  border-radius: 8px;
}

.placeholder-banner {
  width: 100%;
  height: 200px;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: #2c2c2e;
  border-radius: 8px;
  color: #8e8e93;
  font-size: 48px;
}

.game-title-container {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  padding: 20px;
  background: linear-gradient(to top, rgba(0,0,0,0.8), rgba(0,0,0,0));
  border-radius: 0 0 8px 8px;
}

.game-title {
  margin: 0 0 5px 0;
  color: #ffffff;
  font-size: 24px;
}

.game-id {
  margin: 0;
  color: #a1a1a6;
  font-size: 14px;
}

.game-stats {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 15px;
  margin-bottom: 20px;
}

.stat {
  background-color: #2c2c2e;
  border-radius: 8px;
  padding: 12px;
}

.stat-label {
  color: #a1a1a6;
  font-size: 14px;
  margin-bottom: 5px;
}

.stat-value {
  color: #ffffff;
  font-size: 16px;
  font-weight: 500;
}

.game-description {
  margin-bottom: 20px;
}

.game-description h3 {
  font-size: 18px;
  margin-bottom: 10px;
  color: #ffffff;
}

.game-description p {
  color: #a1a1a6;
  line-height: 1.6;
}

.game-tags {
  margin-bottom: 20px;
}

.game-tags h3 {
  font-size: 18px;
  margin-bottom: 10px;
  color: #ffffff;
}

.tags-container {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.tag {
  background-color: #3a3a3c;
  border-radius: 20px;
  padding: 5px 12px;
  font-size: 14px;
  color: #ffffff;
}

.game-screenshots {
  margin-bottom: 20px;
}

.game-screenshots h3 {
  font-size: 18px;
  margin-bottom: 10px;
  color: #ffffff;
}

.screenshots-container {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: 10px;
}

.screenshot {
  width: 100%;
  height: 135px;
  object-fit: cover;
  border-radius: 6px;
  transition: transform 0.3s;
}

.screenshot:hover {
  transform: scale(1.05);
}

.steam-link {
  margin-top: auto;
  padding-top: 20px;
}

.steam-button {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  background-color: #171a21;
  color: #ffffff;
  text-decoration: none;
  padding: 12px 20px;
  border-radius: 8px;
  font-size: 16px;
  font-weight: 500;
  transition: background-color 0.3s;
}

.steam-button:hover {
  background-color: #1b2838;
}
</style> 