
import React, { useState, useEffect } from 'react';

const AddressBook = ({ onSelect }) => {
    const [contacts, setContacts] = useState([]);
    const [name, setName] = useState('');
    const [address, setAddress] = useState('');
    const [showAdd, setShowAdd] = useState(false);

    useEffect(() => {
        const stored = localStorage.getItem('volt_contacts');
        if (stored) setContacts(JSON.parse(stored));
    }, []);

    const saveContact = () => {
        if (!name || !address) return;
        const newContacts = [...contacts, { name, address }];
        setContacts(newContacts);
        localStorage.setItem('volt_contacts', JSON.stringify(newContacts));
        setName('');
        setAddress('');
        setShowAdd(false);
    };

    const deleteContact = (idx) => {
        const newContacts = contacts.filter((_, i) => i !== idx);
        setContacts(newContacts);
        localStorage.setItem('volt_contacts', JSON.stringify(newContacts));
    };

    return (
        <div className="glass-card" style={{ marginTop: '20px', padding: '15px' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '15px' }}>
                <h4 style={{ margin: 0, color: '#94a3b8' }}>📖 Address Book</h4>
                <button className="btn btn-sm btn-secondary" onClick={() => setShowAdd(!showAdd)} style={{ fontSize: '0.8rem', padding: '2px 8px' }}>
                    {showAdd ? 'Cancel' : '+ Add New'}
                </button>
            </div>

            {showAdd && (
                <div style={{ background: 'rgba(0,0,0,0.2)', padding: '10px', borderRadius: '8px', marginBottom: '10px' }}>
                    <input
                        placeholder="Name"
                        value={name}
                        onChange={(e) => setName(e.target.value)}
                        style={{ width: '100%', marginBottom: '5px', background: 'rgba(255,255,255,0.05)', border: 'none', color: 'white', padding: '5px' }}
                    />
                    <input
                        placeholder="Address (Hex)"
                        value={address}
                        onChange={(e) => setAddress(e.target.value)}
                        style={{ width: '100%', marginBottom: '5px', background: 'rgba(255,255,255,0.05)', border: 'none', color: 'white', padding: '5px' }}
                    />
                    <button className="btn btn-primary btn-sm w-100" onClick={saveContact}>Save Contact</button>
                </div>
            )}

            <div style={{ maxHeight: '200px', overflowY: 'auto' }}>
                {contacts.length === 0 ? (
                    <p style={{ color: '#666', fontStyle: 'italic', fontSize: '0.9rem' }}>No contacts saved.</p>
                ) : (
                    contacts.map((c, i) => (
                        <div key={i} style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '8px', borderBottom: '1px solid rgba(255,255,255,0.05)' }}>
                            <div onClick={() => onSelect(c.address)} style={{ cursor: 'pointer', flex: 1 }}>
                                <div style={{ fontWeight: 'bold', fontSize: '0.9rem' }}>{c.name}</div>
                                <div style={{ fontSize: '0.75rem', fontFamily: 'monospace', color: '#666' }}>{c.address.substr(0, 10)}...</div>
                            </div>
                            <button onClick={() => deleteContact(i)} style={{ background: 'none', border: 'none', color: '#ef4444', cursor: 'pointer' }}>×</button>
                        </div>
                    ))
                )}
            </div>
        </div>
    );
};

export default AddressBook;
