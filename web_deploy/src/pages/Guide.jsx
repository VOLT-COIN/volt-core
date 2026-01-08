import React from 'react';

const Guide = () => {
    return (
        <div className="container mx-auto p-6 text-slate-200">
            <div className="max-w-4xl mx-auto bg-slate-800 p-8 rounded-xl shadow-lg border border-slate-700">
                <h1 className="text-4xl font-bold mb-6 text-blue-400">⚡ User Guide & Documentation</h1>

                <div className="bg-red-900/30 border-l-4 border-red-500 p-4 mb-8 rounded">
                    <h3 className="text-xl font-bold text-red-500 mb-2">⚠️ IMPORTANT: SECURITY WARNING</h3>
                    <p className="mb-2">Your coins are stored in a file named <code className="bg-slate-900 px-2 py-1 rounded">wallet.key</code> (or wallet.dat).</p>
                    <ul className="list-disc pl-5 space-y-1 text-red-300">
                        <li>IF YOU DELETE THIS FILE, YOU LOSE YOUR COINS FOREVER.</li>
                        <li>IF YOU OVERWRITE THIS FILE WITH A NEW VERSION, YOU LOSE YOUR COINS FOREVER.</li>
                    </ul>
                    <p className="mt-4 font-bold">🛡️ BACK UP YOUR wallet.key FILE NOW to a safe place!</p>
                </div>

                <section className="mb-8">
                    <h2 className="text-2xl font-bold mb-4 text-white">1. Installation & Updates</h2>
                    <ul className="list-disc pl-5 space-y-2 text-slate-300">
                        <li><strong>New Install:</strong> Unzip the downloaded file to a folder (e.g., C:\Volt).</li>
                        <li><strong>Updating:</strong>
                            <ol className="list-decimal pl-5 mt-2 space-y-1">
                                <li>Backup your existing <code className="text-blue-300">wallet.key</code> first.</li>
                                <li>Delete the old .exe files.</li>
                                <li>Copy the NEW .exe files into the folder.</li>
                                <li><strong>Ensure your original wallet.key is still there.</strong></li>
                            </ol>
                        </li>
                    </ul>
                </section>

                <section className="mb-8">
                    <h2 className="text-2xl font-bold mb-4 text-white">2. How to Start</h2>
                    <ul className="list-disc pl-5 space-y-2 text-slate-300">
                        <li>Run <code className="text-green-400">run_node.bat</code> (or volt_core.exe). Keep the black window OPEN.</li>
                        <li>Run <code className="text-green-400">volt_wallet.exe</code>. It will connect automatically.</li>
                        <li>Wait for Status: <span className="text-green-400 font-bold">Connected</span>.</li>
                    </ul>
                </section>

                <section className="mb-8">
                    <h2 className="text-2xl font-bold mb-4 text-white">3. Wallet Backup & Restore</h2>
                    <div className="space-y-4 text-slate-300">
                        <div>
                            <h4 className="font-bold text-lg text-blue-300">To Backup:</h4>
                            <p>Go to <strong>Settings &rarr; Security &rarr; Show Mnemonic</strong> and write down your 12 words.</p>
                            <p>Or manually copy <code className="text-blue-300">wallet.key</code> to a USB drive.</p>
                        </div>
                        <div>
                            <h4 className="font-bold text-lg text-blue-300">To Restore:</h4>
                            <p><strong>File:</strong> Place your wallet.key in the same folder as the wallet app.</p>
                            <p><strong>Mnemonic:</strong> Open Wallet, click Import, and enter your 12 words.</p>
                        </div>
                    </div>
                </section>

                <section className="mb-8">
                    <h2 className="text-2xl font-bold mb-4 text-white">4. Mining Guide</h2>
                    <p className="mb-2 text-slate-300">To mine Volt (VLT), use an external miner (like cpuminer-opt).</p>
                    <div className="bg-slate-900 p-4 rounded border border-slate-700 font-mono text-sm text-green-400 overflow-x-auto">
                        cpuminer -a sha256d -o stratum+tcp://82.201.143.174:3333 -u &lt;YOUR_ADDRESS&gt; -p x
                    </div>
                </section>

                <section className="mb-8">
                    <h2 className="text-2xl font-bold mb-4 text-white">5. Smart Contracts</h2>
                    <ul className="list-disc pl-5 space-y-2 text-slate-300">
                        <li><strong>Deploying:</strong> Navigate to the <a href="/contracts" className="text-blue-400 underline">Contracts Dashboard</a>. Upload your compiled `.wasm` bytecode hex.</li>
                        <li><strong>Interacting:</strong> Use the "Interact" tab to call functions on deployed contracts. You will need the contract address.</li>
                        <li><strong>Gas:</strong> Every operation requires a Gas fee. Ensure your wallet has sufficient VLT (0.1 VLT recommended for deployments).</li>
                    </ul>
                </section>

                <section>
                    <h2 className="text-2xl font-bold mb-4 text-white">6. Troubleshooting</h2>
                    <ul className="list-disc pl-5 space-y-2 text-slate-300">
                        <li><strong>Connecting... forever:</strong> Check internet, allow Firewall on Port 6000 & 6001.</li>
                        <li><strong>Transaction Failed:</strong> Ensure you have enough balance for fees (0.01 VLT minimum).</li>
                        <li><strong>Lost Address:</strong> You likely deleted wallet.key. Restore using Mnemonic.</li>
                    </ul>
                </section>

            </div>
        </div>
    );
};

export default Guide;
