<template>
  <div class="game-search">
    <div v-if="showingDetails" class="game-details-container">
      <button class="back-button" @click="closeDetails">
        <i class="fas fa-arrow-left"></i> Back to Search
      </button>
      <GameDetails :game="selectedGame" />
    </div>
    
    <div v-else>
      <div class="search-container">
        <input 
          v-model="searchQuery" 
          @input="debouncedSearch"
          @keyup.enter="search"
          type="text" 
          placeholder="Search games by name or AppID (separate multiple terms with comma)..." 
          class="search-input"
        />
        <button @click="search" class="search-button">
          Cari
        </button>
      </div>
      
      <p class="search-tip">
        Tip: Anda dapat mencari berdasarkan nama game atau AppID (pisahkan dengan koma untuk mencari beberapa sekaligus)
      </p>
      
      <div v-if="loading" class="loading">
        <div class="spinner"></div>
        <p>{{ loadingMessage }}</p>
      </div>
      
      <div v-else-if="error" class="error">
        <p>{{ error }}</p>
        <button @click="search" class="retry-button">Retry</button>
      </div>
      
      <div v-else-if="games.length === 0 && searchQuery" class="no-results">
        <p>No games found for "{{ searchQuery }}"</p>
      </div>
      
      <div v-else>
        <div class="search-info" v-if="total > 0">
          <h2 class="results-title">
            Hasil Pencarian: {{ total }} game ditemukan
            <span v-if="hasDlcInfo">(tidak termasuk {{ dlcCount }} DLC)</span>
          </h2>
        </div>
        
        <div class="game-list">
          <game-card 
            v-for="game in games" 
            :key="game.app_id"
            :game="game"
            @select="selectGame"
            @show-details="showDetails"
          />
        </div>
        
        <div v-if="totalPages > 1" class="pagination">
          <button 
            :disabled="currentPage === 1 || loading" 
            @click="changePage(currentPage - 1)"
            class="pagination-button"
          >
            <i class="fas fa-chevron-left"></i> Previous
          </button>
          
          <div class="page-numbers" v-if="totalPages <= 10">
            <button 
              v-for="p in totalPages" 
              :key="p"
              :class="['page-number', { active: p === currentPage, disabled: loading }]"
              @click="changePage(p)"
              :disabled="loading"
            >
              {{ p }}
            </button>
          </div>
          
          <div class="page-numbers" v-else>
            <!-- First page -->
            <button 
              :class="['page-number', { active: 1 === currentPage, disabled: loading }]"
              @click="changePage(1)"
              :disabled="loading"
            >
              1
            </button>
            
            <!-- Ellipsis if current page is far from start -->
            <span v-if="currentPage > 4">...</span>
            
            <!-- Pages around current page -->
            <button 
              v-for="p in getPageRange()" 
              :key="p"
              :class="['page-number', { active: p === currentPage, disabled: loading }]"
              @click="changePage(p)"
              :disabled="loading"
            >
              {{ p }}
            </button>
            
            <!-- Ellipsis if current page is far from end -->
            <span v-if="currentPage < totalPages - 3">...</span>
            
            <!-- Last page -->
            <button 
              :class="['page-number', { active: totalPages === currentPage, disabled: loading }]"
              @click="changePage(totalPages)"
              :disabled="loading"
            >
              {{ totalPages }}
            </button>
          </div>
          
          <span class="page-info">
            Page {{ currentPage }} of {{ totalPages }}
          </span>
          
          <button 
            :disabled="currentPage === totalPages || loading" 
            @click="changePage(currentPage + 1)"
            class="pagination-button"
          >
            Next <i class="fas fa-chevron-right"></i>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script>
import { invoke } from '@tauri-apps/api/tauri';
import GameCard from './GameCard.vue';
import GameDetails from './GameDetails.vue';

export default {
  name: 'GameSearch',
  components: {
    GameCard,
    GameDetails
  },
  data() {
    return {
      searchQuery: '',
      lastSearchQuery: '',
      currentPage: 1,
      games: [],
      total: 0,
      totalPages: 1,
      loading: false,
      loadingMessage: 'Searching...',
      error: null,
      debounceTimeout: null,
      perPage: 20,
      dlcCount: 16,
      hasDlcInfo: false,
      searchResults: null,
      isPageChanging: false,
      selectedGame: null,
      showingDetails: false,
      lastApiCallTimestamp: 0,
      minApiCallInterval: 1000,
    };
  },
  methods: {
    async changePage(page) {
      // Prevent multiple page changes or changing to current page
      if (page === this.currentPage || this.loading || this.isPageChanging) {
        return;
      }
      
      console.log(`Changing to page ${page}`);
      
      // Set page changing flag
      this.isPageChanging = true;
      
      // Update page immediately
      this.currentPage = page;
      
      // If we have complete search results, just update the page
      if (this.searchResults) {
        console.log(`Using cached search results for page ${page}`);
        this.updatePageFromSearchResults();
        this.isPageChanging = false;
        return;
      }
      
      // Clear current games to show loading state immediately
      this.games = [];
      this.loading = true;
      this.loadingMessage = `Loading page ${page}...`;
      
      // Ensure UI updates before search
      await this.$nextTick();
      
      // Fetch data for new page
      await this.search();
    },
    
    async search(forceRefresh = false) {
      // Clear results if search query is empty
      if (!this.searchQuery.trim()) {
        this.resetSearchResults();
        return;
      }
      
      // If forcing refresh or query changed, clear cached results
      if (forceRefresh || this.searchQuery !== this.lastSearchQuery) {
        this.searchResults = null;
      this.lastSearchQuery = this.searchQuery;
      }
      
      // Set loading state if not already set
      if (!this.loading) {
      this.loading = true;
        this.loadingMessage = 'Searching...';
      this.error = null;
      }
      
      try {
        // Only fetch if we don't have results or forcing refresh
        if (!this.searchResults || forceRefresh) {
        console.log(`Fetching results for "${this.searchQuery}" page ${this.currentPage}`);
        
          // Implement rate limiting
          await this.enforceApiRateLimit();
          
          // Fetch results
        const results = await invoke('search_game_by_name', { 
          query: this.searchQuery,
          page: this.currentPage,
          perPage: this.perPage
        });
        
          // Store complete results
          this.searchResults = results;
          
          // Update last API call timestamp
          this.lastApiCallTimestamp = Date.now();
        }
        
        // Update current page from results
        this.updatePageFromSearchResults();
      } catch (error) {
        console.error('Search error:', error);
        this.error = `Failed to search games: ${error}`;
      } finally {
        this.loading = false;
        this.isPageChanging = false;
      }
    },
    
    updatePageFromSearchResults() {
      if (!this.searchResults) return;
      
      this.games = this.searchResults.games;
      this.total = this.searchResults.total;
      this.totalPages = this.searchResults.total_pages;
      this.hasDlcInfo = this.total >= 50;
      
      console.log(`Updated UI with ${this.games.length} games for page ${this.currentPage}`);
    },
    
    resetSearchResults() {
      this.games = [];
      this.total = 0;
      this.totalPages = 1;
      this.currentPage = 1;
      this.searchResults = null;
      this.lastSearchQuery = '';
    },
    
    async enforceApiRateLimit() {
      const now = Date.now();
      const timeSinceLastCall = now - this.lastApiCallTimestamp;
      
      if (timeSinceLastCall < this.minApiCallInterval) {
        const waitTime = this.minApiCallInterval - timeSinceLastCall;
        await new Promise(resolve => setTimeout(resolve, waitTime));
      }
    },
    
    debouncedSearch() {
      clearTimeout(this.debounceTimeout);
      this.debounceTimeout = setTimeout(() => {
        if (this.searchQuery !== this.lastSearchQuery) {
          this.currentPage = 1;
          this.search();
        }
      }, 500);
    },
    
    selectGame(game) {
      this.$emit('select-game', game);
    },
    
    showDetails(gameDetails) {
      this.selectedGame = gameDetails;
      this.showingDetails = true;
    },
    
    closeDetails() {
      this.showingDetails = false;
      this.selectedGame = null;
    },
    
    getPageRange() {
      const current = this.currentPage;
      const total = this.totalPages;
      
      if (current <= 4) {
        // Near beginning
        return Array.from({length: Math.min(5, total - 1)}, (_, i) => i + 2);
      } else if (current >= total - 3) {
        // Near end
        return Array.from({length: Math.min(5, total - 1)}, (_, i) => total - 5 + i);
      } else {
        // Middle
        return [current - 2, current - 1, current, current + 1, current + 2];
      }
    },
    
    async initializeDatabase() {
      try {
        await invoke('initialize_database');
      } catch (error) {
        console.error('Failed to initialize database:', error);
      }
    },
  },
  mounted() {
    this.initializeDatabase();
    
    if (this.searchQuery) {
      this.search();
    }
  },
  watch: {
    currentPage(newPage, oldPage) {
      console.log(`Page changed from ${oldPage} to ${newPage}`);
    }
  }
};
</script>

<style scoped>
.game-search {
  width: 100%;
  max-width: 1200px;
  margin: 0 auto;
  padding: 20px;
}

.game-details-container {
  width: 100%;
  margin-bottom: 20px;
}

.back-button {
  margin-bottom: 20px;
  padding: 10px 15px;
  background-color: #2c2c2e;
  border: none;
  border-radius: 8px;
  color: white;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 8px;
  transition: background-color 0.3s;
}

.back-button:hover {
  background-color: #3a3a3c;
}

.search-container {
  display: flex;
  margin-bottom: 20px;
}

.search-input {
  flex: 1;
  padding: 12px 15px;
  border: 2px solid #3a3a3c;
  border-radius: 8px 0 0 8px;
  background-color: #1c1c1e;
  color: #ffffff;
  font-size: 16px;
}

.search-button {
  padding: 12px 20px;
  background-color: #6366f1;
  border: none;
  border-radius: 0 8px 8px 0;
  color: white;
  cursor: pointer;
  transition: background-color 0.3s;
  font-weight: 500;
  min-width: 80px;
  text-align: center;
}

.search-button:hover {
  background-color: #4f46e5;
}

.search-tip {
  margin-bottom: 15px;
  color: #a1a1a6;
  font-size: 14px;
}

.search-info {
  margin-bottom: 20px;
}

.results-title {
  font-size: 18px;
  font-weight: 600;
  color: #ffffff;
}

.game-list {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
  gap: 20px;
}

.loading, .error, .no-results {
  text-align: center;
  padding: 40px;
  color: #a1a1a6;
}

.spinner {
  border: 4px solid rgba(255, 255, 255, 0.1);
  border-radius: 50%;
  border-top: 4px solid #0071e3;
  width: 40px;
  height: 40px;
  animation: spin 1s linear infinite;
  margin: 0 auto 20px;
}

@keyframes spin {
  0% { transform: rotate(0deg); }
  100% { transform: rotate(360deg); }
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

.pagination {
  display: flex;
  justify-content: center;
  align-items: center;
  margin-top: 30px;
  gap: 15px;
  flex-wrap: wrap;
}

.pagination-button {
  padding: 8px 16px;
  background-color: #2c2c2e;
  border: none;
  border-radius: 6px;
  color: white;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 5px;
}

.pagination-button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.page-info {
  color: #a1a1a6;
}

.page-numbers {
  display: flex;
  gap: 5px;
}

.page-number {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  background-color: #2c2c2e;
  border: none;
  color: white;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
}

.page-number.active {
  background-color: #0071e3;
}

.page-number.disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style> 