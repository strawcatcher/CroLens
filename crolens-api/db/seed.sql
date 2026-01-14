INSERT INTO protocols (protocol_id, name, adapter_type, category, website) VALUES
('vvs', 'VVS Finance', 'uniswap_v2_amm', 'dex', 'https://vvs.finance'),
('tectonic', 'Tectonic', 'compound_v2_lending', 'lending', 'https://tectonic.finance')
ON CONFLICT(protocol_id) DO UPDATE SET
  name = excluded.name,
  adapter_type = excluded.adapter_type,
  category = excluded.category,
  website = excluded.website;

INSERT INTO protocol_contracts (protocol_id, contract_type, address, chain_id) VALUES
('vvs', 'router', '0x145863Eb42Cf62847A6Ca784e6416C1682b1b2Ae', 25),
('vvs', 'factory', '0x3B44B2a187a7b3824131F8db5a74194D0a42Fc15', 25),
('vvs', 'masterchef', '0x3790f3A1cf8A478042Ec112A70881Dcfa9c0fc21', 25),
('vvs', 'reward_token', '0x2D03bece6747ADC00E1a131BBA1469C15fD11e03', 25),
('tectonic', 'comptroller', '0x7De56Bd8b37827c51835e162c867848fE2403a48', 25)
ON CONFLICT(protocol_id, contract_type, chain_id) DO UPDATE SET
  address = excluded.address;

INSERT INTO dex_pools (pool_id, protocol_id, pool_index, lp_address, token0_address, token1_address, token0_symbol, token1_symbol) VALUES
('vvs_cro_usdc', 'vvs', 0, '0xe61Db569E231B3f5530168Aa2C9D50246525b6d6', '0x5C7F8A570d578ED84E63fdFA7b1eE72dEae1AE23', '0xc21223249CA28397B4B6541dfFaEcC539BfF0c59', 'WCRO', 'USDC'),
('vvs_cro_usdt', 'vvs', 1, '0x3d2180DB9E1B909f35C398BC39EF36108C0FC8c3', '0x5C7F8A570d578ED84E63fdFA7b1eE72dEae1AE23', '0x66e428c3f67a68878562e79A0234c1F83c208770', 'WCRO', 'USDT'),
('vvs_cro_eth', 'vvs', 2, '0xA111C17f8B8303280d3EB01BBcd61000AA7F39F9', '0x5C7F8A570d578ED84E63fdFA7b1eE72dEae1AE23', '0xe44Fd7fCb2b1581822D0c862B68222998a0c299a', 'WCRO', 'WETH'),
('vvs_usdc_usdt', 'vvs', 3, '0x39cC0E14795A8e6e9D02A21091b81FE0d61D82f9', '0xc21223249CA28397B4B6541dfFaEcC539BfF0c59', '0x66e428c3f67a68878562e79A0234c1F83c208770', 'USDC', 'USDT'),
('vvs_vvs_cro', 'vvs', 4, '0xbf62c67eA509E86F07c8c69d0286C0636C50270b', '0x2D03bece6747ADC00E1a131BBA1469C15fD11e03', '0x5C7F8A570d578ED84E63fdFA7b1eE72dEae1AE23', 'VVS', 'WCRO')
ON CONFLICT(pool_id) DO UPDATE SET
  protocol_id = excluded.protocol_id,
  pool_index = excluded.pool_index,
  lp_address = excluded.lp_address,
  token0_address = excluded.token0_address,
  token1_address = excluded.token1_address,
  token0_symbol = excluded.token0_symbol,
  token1_symbol = excluded.token1_symbol;

INSERT INTO lending_markets (market_id, protocol_id, ctoken_address, underlying_address, underlying_symbol, collateral_factor) VALUES
('tectonic_cro', 'tectonic', '0xeAdf7c01DA7E93FdB5f16B0aa9ee85f978e89E95', '0x5C7F8A570d578ED84E63fdFA7b1eE72dEae1AE23', 'WCRO', '0.75'),
('tectonic_usdc', 'tectonic', '0xb3bbf1be947b245aef26e3b6a9d777d7703f4c8e', '0xc21223249CA28397B4B6541dfFaEcC539BfF0c59', 'USDC', '0.85'),
('tectonic_usdt', 'tectonic', '0xa683fdfd9286eedfea81cf6da14703da683c44e5', '0x66e428c3f67a68878562e79A0234c1F83c208770', 'USDT', '0.85'),
('tectonic_weth', 'tectonic', '0x543F4Db9BD26C9Eb6aD4DD1C33522c966C625774', '0xe44Fd7fCb2b1581822D0c862B68222998a0c299a', 'WETH', '0.80')
ON CONFLICT(market_id) DO UPDATE SET
  protocol_id = excluded.protocol_id,
  ctoken_address = excluded.ctoken_address,
  underlying_address = excluded.underlying_address,
  underlying_symbol = excluded.underlying_symbol,
  collateral_factor = excluded.collateral_factor;

INSERT INTO contracts (address, name, type, protocol_id) VALUES
('0x145863Eb42Cf62847A6Ca784e6416C1682b1b2Ae', 'VVS Router', 'DEX Router', 'vvs'),
('0x3B44B2a187a7b3824131F8db5a74194D0a42Fc15', 'VVS Factory', 'DEX Factory', 'vvs'),
('0x3790f3A1cf8A478042Ec112A70881Dcfa9c0fc21', 'VVS MasterChef', 'Farm', 'vvs'),
('0x7De56Bd8b37827c51835e162c867848fE2403a48', 'Tectonic Comptroller', 'Lending Core', 'tectonic'),
('0xcA11bde05977b3631167028862bE2a173976CA11', 'Multicall3', 'Utility', NULL),
('0x5C7F8A570d578ED84E63fdFA7b1eE72dEae1AE23', 'WCRO', 'Token', NULL),
('0xc21223249CA28397B4B6541dfFaEcC539BfF0c59', 'USDC', 'Token', NULL),
('0x2D03bece6747ADC00E1a131BBA1469C15fD11e03', 'VVS Token', 'Token', 'vvs'),
('0xDD73dEa10ABC2Bff99c60882EC5b2B81bb1Dc5B2', 'TONIC Token', 'Token', 'tectonic')
ON CONFLICT(address) DO UPDATE SET
  name = excluded.name,
  type = excluded.type,
  protocol_id = excluded.protocol_id;

INSERT INTO tokens (address, symbol, name, decimals, is_stablecoin, coingecko_id, is_anchor) VALUES
('0x5C7F8A570d578ED84E63fdFA7b1eE72dEae1AE23', 'WCRO', 'Wrapped CRO', 18, 0, 'crypto-com-chain', 1),
('0xc21223249CA28397B4B6541dfFaEcC539BfF0c59', 'USDC', 'USD Coin', 6, 1, 'usd-coin', 1),
('0x66e428c3f67a68878562e79A0234c1F83c208770', 'USDT', 'Tether USD', 6, 1, 'tether', 1),
('0xe44Fd7fCb2b1581822D0c862B68222998a0c299a', 'WETH', 'Wrapped Ether', 18, 0, 'ethereum', 1),
('0x062E66477Faf219F25D27dCED647BF57C3107d52', 'WBTC', 'Wrapped BTC', 8, 0, 'bitcoin', 1),
('0xF2001B145b43032AAF5Ee2884e456CCd805F677D', 'DAI', 'Dai Stablecoin', 18, 1, 'dai', 1),
('0x2D03bece6747ADC00E1a131BBA1469C15fD11e03', 'VVS', 'VVS Token', 18, 0, 'vvs-finance', 1),
('0xDD73dEa10ABC2Bff99c60882EC5b2B81bb1Dc5B2', 'TONIC', 'Tectonic', 18, 0, 'tectonic', 1),
('0xB888d8Dd1733d72681b30c00ee76BDE93ae7aa93', 'ATOM', 'Cosmos', 6, 0, 'cosmos', 1)
ON CONFLICT(address) DO UPDATE SET
  symbol = excluded.symbol,
  name = excluded.name,
  decimals = excluded.decimals,
  is_stablecoin = excluded.is_stablecoin,
  coingecko_id = excluded.coingecko_id,
  is_anchor = excluded.is_anchor;

INSERT INTO system_config (key, value, value_type, description) VALUES
('x402.price_per_credit', '10000000000000000', 'string', 'Price per credit (0.01 CRO)'),
('x402.free_daily_limit', '50', 'int', 'Free tier daily requests'),
('feature.simulation', 'true', 'bool', 'Enable simulation'),
('rpc.max_retries', '3', 'int', 'Max RPC retries')
ON CONFLICT(key) DO UPDATE SET
  value = excluded.value,
  value_type = excluded.value_type,
  description = excluded.description;
