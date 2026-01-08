export const getApiConfig = () => {
    let url = localStorage.getItem('volt_node_url') || 'http://82.201.143.174';
    // Normalize URL: Ensure http:// or https://
    if (url && !url.startsWith('http')) {
        url = 'http://' + url;
    }
    return {
        params: { node: url }, // Send as ?node=...
        headers: { 'X-Node-Url': url }
    };
};

// Start Helper for POST payloads with password
export const getPayload = (cmd, data = {}) => ({
    command: cmd,
    password: localStorage.getItem('rpc_password') || undefined,
    ...data
});
