<template>
  <div class="app">
    <header class="header">
      <h1 class="app-title">Game List</h1>
      <button class="theme-toggle">
        <i class="fas fa-moon"></i>
      </button>
    </header>
    
    <main class="main-content">
      <GameSearch @select-game="selectGame" />
    </main>
  </div>
</template>

<script>
import GameSearch from './components/GameSearch.vue';

export default {
  name: 'App',
  components: {
    GameSearch
  },
  data() {
    return {
      selectedGame: null
    };
  },
  methods: {
    selectGame(game) {
      this.selectedGame = game;
    }
  }
};
</script>

<style>
@import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap');
@import url('https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.4.0/css/all.min.css');

:root {
  --background-color: #000000;
  --card-background: #1c1c1e;
  --secondary-background: #2c2c2e;
  --border-color: #3a3a3c;
  --text-color: #ffffff;
  --secondary-text: #a1a1a6;
  --accent-color: #6366f1;
  --accent-hover: #4f46e5;
  --success-color: #32d74b;
  --error-color: #ff453a;
}

* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, 'Open Sans', 'Helvetica Neue', sans-serif;
  background-color: var(--background-color);
  color: var(--text-color);
  line-height: 1.6;
}

.app {
  display: flex;
  flex-direction: column;
  min-height: 100vh;
}

.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 20px 40px;
}

.app-title {
  font-size: 28px;
  font-weight: 700;
}

.theme-toggle {
  width: 40px;
  height: 40px;
  border-radius: 50%;
  background-color: transparent;
  border: none;
  color: var(--text-color);
  font-size: 20px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background-color 0.3s;
}

.theme-toggle:hover {
  background-color: var(--secondary-background);
}

.main-content {
  flex: 1;
  padding: 0 20px 40px;
}
</style> 