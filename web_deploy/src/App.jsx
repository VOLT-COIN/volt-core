import { BrowserRouter as Router, Routes, Route } from 'react-router-dom';
import Layout from './components/Layout';
import Home from './pages/Home';
import Explorer from './pages/Explorer';
import BlockDetail from './pages/BlockDetail';
import TxDetail from './pages/TxDetail';
import AddressDetail from './pages/AddressDetail';
import Wallet from './pages/Wallet';
import Assets from './pages/Assets';
import Staking from './pages/Staking';
import Dex from './pages/Dex';
import Swap from './pages/Swap';
import Pool from './pages/Pool';
import Nft from './pages/Nft';
import Whitepaper from './pages/Whitepaper';
import Privacy from './pages/Privacy';
import Terms from './pages/Terms';
import Status from './pages/Status';
import Guide from './pages/Guide';
import Contracts from './pages/Contracts';
import './App.css';

function App() {
  return (
    <Router>
      <Layout>
        <Routes>
          <Route path="/" element={<Home />} />
          <Route path="/explorer" element={<Explorer />} />
          <Route path="/block/:id" element={<BlockDetail />} />
          <Route path="/tx/:id" element={<TxDetail />} />
          <Route path="/address/:id" element={<AddressDetail />} />
          <Route path="/wallet" element={<Wallet />} />
          <Route path="/assets" element={<Assets />} />
          <Route path="/staking" element={<Staking />} />
          <Route path="/dex" element={<Dex />} />
          <Route path="/swap" element={<Swap />} />
          <Route path="/pool" element={<Pool />} />
          <Route path="/nft" element={<Nft />} />
          <Route path="/whitepaper" element={<Whitepaper />} />
          <Route path="/privacy" element={<Privacy />} />
          <Route path="/terms" element={<Terms />} />
          <Route path="/status" element={<Status />} />
          <Route path="/guide" element={<Guide />} />
          <Route path="/contracts" element={<Contracts />} />
        </Routes>
      </Layout>
    </Router>
  );
}

export default App;
