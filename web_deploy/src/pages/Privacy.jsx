function Privacy() {
    return (
        <div className="container" style={{ paddingTop: '100px', color: '#ccc' }}>
            <h1>Privacy Policy</h1>
            <p>Last Updated: January 2026</p>

            <h3>1. Introduction</h3>
            <p>
                Volt ("we", "our", or "us") respects your privacy. This policy describes how we handle information in relation to the Volt blockchain software and website.
            </p>

            <h3>2. No Data Collection</h3>
            <p>
                The Volt software (Node and Wallet) is designed to run locally on your machine.
                <strong>We do not collect, store, or transmit your private keys, seed phrases, or personal data to any central server.</strong>
                All transaction data is public on the blockchain, as is the nature of decentralized networks.
            </p>

            <h3>3. Website Analytics</h3>
            <p>
                This website may use basic, anonymized analytics (like Vercel Analytics) to track performance and uptime. No personally identifiable information (PII) is harvested.
            </p>

            <h3>4. Changes to This Policy</h3>
            <p>
                We may update this privacy policy from time to time. Promoting decentralization means handling as little user data as possible.
            </p>

            <div style={{ marginTop: '50px' }}>
                <a href="/" className="btn btn-secondary">← Back to Home</a>
            </div>
        </div>
    );
}

export default Privacy;
