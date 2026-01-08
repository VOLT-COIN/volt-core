function Terms() {
    return (
        <div className="container" style={{ paddingTop: '100px', color: '#ccc' }}>
            <h1>Terms of Service</h1>
            <p>Last Updated: January 2026</p>

            <h3>1. Acceptance of Terms</h3>
            <p>
                By downloading or using the Volt software, you agree to these terms. If you do not agree, do not use the software.
            </p>

            <h3>2. Disclaimer of Warranties</h3>
            <p>
                The Volt software is provided "AS IS", without warranty of any kind, express or implied.
                <strong>You use the software entirely at your own risk.</strong>
                The developers are not responsible for any financial loss, data loss, or other damages arising from the use of this software.
            </p>

            <h3>3. Open Source License</h3>
            <p>
                Volt is open-source software. You are free to modify and distribute it under the terms of its license (MIT/Apache 2.0).
            </p>

            <h3>4. Regulatory Compliance</h3>
            <p>
                You are responsible for ensuring that your use of cryptocurrency complies with the laws and regulations of your local jurisdiction.
            </p>

            <div style={{ marginTop: '50px' }}>
                <a href="/" className="btn btn-secondary">← Back to Home</a>
            </div>
        </div>
    );
}

export default Terms;
