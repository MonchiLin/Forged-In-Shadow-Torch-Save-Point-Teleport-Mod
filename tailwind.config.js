/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{vue,js,ts,jsx,tsx}'],
  theme: {
    extend: {
      colors: {
        accent: '#bb5141',
        'accent-strong': '#d96a58',
        'accent-muted': 'rgba(187, 81, 65, 0.14)',
        'device-border': 'rgba(255, 255, 255, 0.05)',
        'text-strong': '#f5f7fb',
        'text-body': 'rgba(214, 218, 232, 0.9)',
        'text-muted': 'rgba(160, 170, 194, 0.65)',
      },
      fontFamily: {
        display: ['Inter', 'Segoe UI', 'Microsoft YaHei', 'sans-serif'],
      },
      boxShadow: {
        marker: '0 0 0 6px rgba(187, 81, 65, 0.2), 0 18px 36px rgba(187, 81, 65, 0.32)',
      },
      backgroundImage: {
        'bg-grid': "url('data:image/svg+xml,%3Csvg xmlns=\'http://www.w3.org/2000/svg\' width=\'160\' height=\'160\'%3E%3Cfilter id=\'n\'%3E%3CfeTurbulence type=\'fractalNoise\' baseFrequency=\'0.75\' numOctaves=\'4\' stitchTiles=\'stitch\'/%3E%3C/filter%3E%3Crect width=\'160\' height=\'160\' filter=\'url(%23n)\' opacity=\'0.18\'/%3E%3C/svg%3E')",
      },
    },
  },
  plugins: [],
};
