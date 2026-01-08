async function check() {
    try {
        const res = await fetch('https://voltcore-node.hf.space/rpc', {
            method: 'POST',
            body: JSON.stringify({ command: 'get_chain_info' }),
            headers: { 'Content-Type': 'application/json' }
        });
        const data = await res.json();
        console.log('Chain Info:', JSON.stringify(data, null, 2));

        const height = data.data.height;
        console.log('Fetching blocks starting from:', height - 10);
        const bRes = await fetch('https://voltcore-node.hf.space/rpc', {
            method: 'POST',
            body: JSON.stringify({
                command: 'get_blocks',
                start_index: Math.max(0, height - 10)
            }),
            headers: { 'Content-Type': 'application/json' }
        });
        const bData = await bRes.json();
        console.log('Blocks Data:', JSON.stringify(bData, null, 2));

    } catch (e) {
        console.error('Error:', e.message);
    }
}
check();
