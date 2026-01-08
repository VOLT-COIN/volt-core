import { Link } from 'react-router-dom';

function Layout({ children }) {
    return (
        <div className="container">
            <nav className="navbar">
                <div className="logo">
                    <img src="/logo.png?v=2" alt="Volt Logo" style={{ height: '40px', verticalAlign: 'middle', marginRight: '10px' }} />
                    VOLT COIN
                </div>
                <div className="nav-links">
                    <Link to="/">Home</Link>
                    <Link to="/explorer">Explorer</Link>
                    <Link to="/wallet">Web Wallet</Link>
                    <Link to="/assets">Assets</Link>
                    <Link to="/staking">Staking</Link>
                    <Link to="/dex">OrderBook</Link>
                    <Link to="/swap">Swap</Link>
                    <Link to="/pool">Pools</Link>
                    <Link to="/nft" className="nav-link" style={{ color: '#d4af37' }}>NFTs</Link>
                    <Link to="/contracts" style={{ color: '#ec4899' }}>Contracts</Link>
                    <Link to="/guide" style={{ color: '#60a5fa' }}>Guide</Link>
                </div>
            </nav>
            <main>
                {children}
            </main>
            <footer>
                <p>&copy; 2026 Volt Foundation. Open Source. <span style={{ margin: '0 10px' }}>|</span> <Link to="/status" style={{ color: '#10b981', textDecoration: 'none' }}>System Status ●</Link></p>
            </footer>
        </div>
    );
}

export default Layout;
