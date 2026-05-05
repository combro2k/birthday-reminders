/** @type {import('tailwindcss').Config} */
module.exports = {
  darkMode: 'selector',
  content: ["./templates/**/*.html"],
  theme: {
    extend: {
      colors: {
        brand: {
          50: "#f0fdfa",
          100: "#ccfbf1",
          200: "#99f6e4",
          300: "#5eead4",
          400: "#2dd4bf",
          500: "#14b8a6",
          600: "#0d9488",
          700: "#0f766e",
          800: "#115e59",
          900: "#134e4a",
          950: "#042f2e"
        },
        accent: {
          50: "#fefce8",
          100: "#fef9c3",
          500: "#eab308",
          700: "#a16207"
        }
      },
      fontFamily: {
        display: ["Space Grotesk", "ui-sans-serif", "sans-serif"],
        sans: ["Manrope", "ui-sans-serif", "sans-serif"],
        mono: ["JetBrains Mono", "ui-monospace", "monospace"]
      },
      boxShadow: {
        glow: "0 10px 35px rgba(20, 184, 166, 0.24)",
      }
    }
  },
  plugins: [],
};
