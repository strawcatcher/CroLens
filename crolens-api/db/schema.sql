CREATE TABLE IF NOT EXISTS protocols (
    protocol_id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    adapter_type TEXT NOT NULL,
    category TEXT NOT NULL,
    website TEXT,
    logo_url TEXT,
    is_active BOOLEAN DEFAULT 1,
    config TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS protocol_contracts (
    protocol_id TEXT NOT NULL,
    contract_type TEXT NOT NULL,
    address TEXT NOT NULL,
    chain_id INTEGER DEFAULT 25,
    PRIMARY KEY (protocol_id, contract_type, chain_id),
    FOREIGN KEY (protocol_id) REFERENCES protocols(protocol_id)
);

CREATE TABLE IF NOT EXISTS dex_pools (
    pool_id TEXT PRIMARY KEY,
    protocol_id TEXT NOT NULL,
    pool_index INTEGER,
    lp_address TEXT NOT NULL,
    token0_address TEXT NOT NULL,
    token1_address TEXT NOT NULL,
    token0_symbol TEXT,
    token1_symbol TEXT,
    created_at_block INTEGER,
    is_active BOOLEAN DEFAULT 1,
    FOREIGN KEY (protocol_id) REFERENCES protocols(protocol_id)
);
CREATE INDEX IF NOT EXISTS idx_dex_pools_protocol ON dex_pools(protocol_id);

CREATE TABLE IF NOT EXISTS lending_markets (
    market_id TEXT PRIMARY KEY,
    protocol_id TEXT NOT NULL,
    ctoken_address TEXT NOT NULL,
    underlying_address TEXT NOT NULL,
    underlying_symbol TEXT,
    collateral_factor TEXT,
    is_active BOOLEAN DEFAULT 1,
    FOREIGN KEY (protocol_id) REFERENCES protocols(protocol_id)
);

CREATE TABLE IF NOT EXISTS tokens (
    address TEXT PRIMARY KEY,
    symbol TEXT NOT NULL,
    name TEXT,
    decimals INTEGER NOT NULL,
    logo_url TEXT,
    is_stablecoin BOOLEAN DEFAULT 0,
    coingecko_id TEXT,
    is_anchor BOOLEAN DEFAULT 0
);

CREATE TABLE IF NOT EXISTS contracts (
    address TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    type TEXT,
    protocol_id TEXT,
    verified BOOLEAN DEFAULT 0,
    description TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_contracts_name ON contracts(name);

CREATE TABLE IF NOT EXISTS system_config (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    value_type TEXT DEFAULT 'string',
    description TEXT,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS api_keys (
    api_key TEXT PRIMARY KEY,
    owner_address TEXT,
    tier TEXT DEFAULT 'free',
    credits INTEGER DEFAULT 50,
    daily_used INTEGER DEFAULT 0,
    daily_reset_at TEXT,
    is_active BOOLEAN DEFAULT 1,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS request_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    trace_id TEXT NOT NULL,
    api_key TEXT,
    tool_name TEXT NOT NULL,
    latency_ms INTEGER,
    status TEXT DEFAULT 'success',
    error_code INTEGER,
    ip_address TEXT,
    request_size INTEGER,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_request_logs_trace ON request_logs(trace_id);
CREATE INDEX IF NOT EXISTS idx_request_logs_apikey ON request_logs(api_key, created_at);

CREATE TABLE IF NOT EXISTS payments (
    tx_hash TEXT PRIMARY KEY,
    api_key TEXT NOT NULL,
    from_address TEXT,
    to_address TEXT,
    value_wei TEXT,
    credits_granted INTEGER,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (api_key) REFERENCES api_keys(api_key)
);
CREATE INDEX IF NOT EXISTS idx_payments_apikey ON payments(api_key, created_at);
