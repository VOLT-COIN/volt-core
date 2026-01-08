// Global Configuration
// VITE_API_URL should be set in .env or Vercel Environment Variables
// Failover to provided Ngrok URL for production access
// Failover to local proxy for Vercel
// Prioritize Environment Variable, fallback to production URL
export const API_URL = import.meta.env.VITE_API_URL || "https://voltcore-node.hf.space/api/rpc"; 
