/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        primary: "#6C63FF",
        background: "#0e0e1a",
        surface: "#16162a",
        sidebar: "#12121e",
        border: "#1e1e30",
        highlight: "#4f46e5",
      },
      fontFamily: {
        sans: ['Inter', 'system-ui', 'sans-serif'],
      },
      boxShadow: {
        'card': '0 8px 30px rgba(0, 0, 0, 0.12)',
      }
    },
  },
  plugins: [],
} 