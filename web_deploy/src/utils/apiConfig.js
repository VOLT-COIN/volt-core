// Target the HF Space directly (It runs Nginx -> Volt Core internally)
let url = 'https://voltcore-node.hf.space';

// If we are pointing to the Proxy itself, we don't need the ?node param
// The Proxy (Nginx) inside the Space forwards /api/rpc to 127.0.0.1:7862
return {
    baseURL: url, // Axios will prepend this to '/api/rpc'
    headers: { 'Content-Type': 'application/json' }
};
};

// Start Helper for POST payloads with password
export const getPayload = (cmd, data = {}) => ({
    command: cmd,
    password: localStorage.getItem('rpc_password') || undefined,
    ...data
});
