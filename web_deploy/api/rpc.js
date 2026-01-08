
import axios from 'axios';

// The address of your Home PC Node
const NODE_URL = 'http://volt-core.zapto.org:6001';

export default async function handler(req, res) {
    // CORS Handling
    res.setHeader('Access-Control-Allow-Credentials', true);
    res.setHeader('Access-Control-Allow-Origin', '*');
    res.setHeader('Access-Control-Allow-Methods', 'GET,OPTIONS,PATCH,DELETE,POST,PUT');
    res.setHeader(
        'Access-Control-Allow-Headers',
        'X-CSRF-Token, X-Requested-With, Accept, Accept-Version, Content-Length, Content-MD5, Content-Type, Date, X-Api-Version, X-Node-Url'
    );

    if (req.method === 'OPTIONS') {
        res.status(200).end();
        return;
    }

    // Dynamic Node Selection
    // 1. Check Header (X-Node-Url)
    // 2. Check Query Param (?node=...)
    // 3. Fallback to Env Var
    // 4. Fallback to Default
    let targetNode = req.headers['x-node-url'] || req.query.node || process.env.VOLT_NODE_URL || 'http://volt-core.zapto.org:6001';

    try {
        // console.log(`[Vercel Proxy] Forwarding to ${targetNode}`);
        const response = await axios.post(targetNode, req.body, { timeout: 15000 });
        res.status(200).json(response.data);
    } catch (error) {
        console.error(`[Proxy Error] Target: ${targetNode}`, error.message);
        res.status(500).json({
            error: "Node unreachable",
            details: error.message,
            target: targetNode,
            hint: "Check URL in Settings or port forwarding."
        });
    }
}
