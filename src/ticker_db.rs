#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TickerInfo {
    pub symbol: &'static str,
    pub name: &'static str,
    pub sector: &'static str,
}

pub const ALL_TICKERS: &[TickerInfo] = &[
    // Mega Cap / Tech
    TickerInfo { symbol: "AAPL", name: "Apple Inc.", sector: "Technology" },
    TickerInfo { symbol: "MSFT", name: "Microsoft Corporation", sector: "Technology" },
    TickerInfo { symbol: "NVDA", name: "NVIDIA Corporation", sector: "Semiconductors" },
    TickerInfo { symbol: "GOOGL", name: "Alphabet Inc. (Class A)", sector: "Communication" },
    TickerInfo { symbol: "GOOG", name: "Alphabet Inc. (Class C)", sector: "Communication" },
    TickerInfo { symbol: "AMZN", name: "Amazon.com, Inc.", sector: "Consumer Cyclical" },
    TickerInfo { symbol: "META", name: "Meta Platforms, Inc.", sector: "Communication" },
    TickerInfo { symbol: "TSLA", name: "Tesla, Inc.", sector: "Automotive" },
    TickerInfo { symbol: "AVGO", name: "Broadcom Inc.", sector: "Semiconductors" },
    TickerInfo { symbol: "ORCL", name: "Oracle Corporation", sector: "Technology" },
    TickerInfo { symbol: "CRM", name: "Salesforce, Inc.", sector: "Technology" },
    TickerInfo { symbol: "AMD", name: "Advanced Micro Devices", sector: "Semiconductors" },
    TickerInfo { symbol: "ADBE", name: "Adobe Inc.", sector: "Technology" },
    TickerInfo { symbol: "CSCO", name: "Cisco Systems, Inc.", sector: "Technology" },
    TickerInfo { symbol: "INTC", name: "Intel Corporation", sector: "Semiconductors" },
    TickerInfo { symbol: "QCOM", name: "QUALCOMM Incorporated", sector: "Semiconductors" },
    TickerInfo { symbol: "TXN", name: "Texas Instruments", sector: "Semiconductors" },
    TickerInfo { symbol: "IBM", name: "IBM Corporation", sector: "Technology" },
    TickerInfo { symbol: "NOW", name: "ServiceNow, Inc.", sector: "Technology" },
    TickerInfo { symbol: "AMAT", name: "Applied Materials, Inc.", sector: "Semiconductors" },
    TickerInfo { symbol: "INTU", name: "Intuit Inc.", sector: "Technology" },
    TickerInfo { symbol: "MU", name: "Micron Technology", sector: "Semiconductors" },
    TickerInfo { symbol: "LRCX", name: "Lam Research Corp.", sector: "Semiconductors" },
    TickerInfo { symbol: "ADI", name: "Analog Devices, Inc.", sector: "Semiconductors" },
    TickerInfo { symbol: "PANW", name: "Palo Alto Networks", sector: "Cybersecurity" },
    TickerInfo { symbol: "PLTR", name: "Palantir Technologies", sector: "Software / AI" },
    TickerInfo { symbol: "UBER", name: "Uber Technologies", sector: "Transportation" },
    TickerInfo { symbol: "ABNB", name: "Airbnb, Inc.", sector: "Consumer Services" },
    TickerInfo { symbol: "SHOP", name: "Shopify Inc.", sector: "E-Commerce" },
    TickerInfo { symbol: "COIN", name: "Coinbase Global, Inc.", sector: "Crypto / Fintech" },
    TickerInfo { symbol: "PYPL", name: "PayPal Holdings", sector: "Fintech" },
    TickerInfo { symbol: "SQ", name: "Block, Inc.", sector: "Fintech" },
    TickerInfo { symbol: "SNPS", name: "Synopsys, Inc.", sector: "Semiconductors" },
    TickerInfo { symbol: "CDNS", name: "Cadence Design Systems", sector: "Semiconductors" },
    TickerInfo { symbol: "KLAC", name: "KLA Corporation", sector: "Semiconductors" },
    TickerInfo { symbol: "CRWD", name: "CrowdStrike Holdings", sector: "Cybersecurity" },
    TickerInfo { symbol: "MRVL", name: "Marvell Technology", sector: "Semiconductors" },
    TickerInfo { symbol: "DELL", name: "Dell Technologies", sector: "Hardware" },
    TickerInfo { symbol: "SMCI", name: "Super Micro Computer", sector: "Hardware" },
    TickerInfo { symbol: "ANET", name: "Arista Networks", sector: "Networking" },

    // Financials
    TickerInfo { symbol: "BRK-B", name: "Berkshire Hathaway (Class B)", sector: "Financial Services" },
    TickerInfo { symbol: "JPM", name: "JPMorgan Chase & Co.", sector: "Banking" },
    TickerInfo { symbol: "V", name: "Visa Inc.", sector: "Payment Services" },
    TickerInfo { symbol: "MA", name: "Mastercard Incorporated", sector: "Payment Services" },
    TickerInfo { symbol: "BAC", name: "Bank of America Corp.", sector: "Banking" },
    TickerInfo { symbol: "WFC", name: "Wells Fargo & Company", sector: "Banking" },
    TickerInfo { symbol: "C", name: "Citigroup Inc.", sector: "Banking" },
    TickerInfo { symbol: "GS", name: "Goldman Sachs Group", sector: "Investment Banking" },
    TickerInfo { symbol: "MS", name: "Morgan Stanley", sector: "Investment Banking" },
    TickerInfo { symbol: "BLK", name: "BlackRock, Inc.", sector: "Asset Management" },
    TickerInfo { symbol: "SCHW", name: "Charles Schwab Corp.", sector: "Brokerage" },
    TickerInfo { symbol: "AXP", name: "American Express Company", sector: "Credit Services" },
    TickerInfo { symbol: "CB", name: "Chubb Limited", sector: "Insurance" },
    TickerInfo { symbol: "PGR", name: "Progressive Corporation", sector: "Insurance" },
    TickerInfo { symbol: "MMC", name: "Marsh & McLennan", sector: "Insurance" },
    TickerInfo { symbol: "AON", name: "Aon plc", sector: "Insurance" },
    TickerInfo { symbol: "USB", name: "U.S. Bancorp", sector: "Banking" },
    TickerInfo { symbol: "PNC", name: "PNC Financial Services", sector: "Banking" },
    TickerInfo { symbol: "TRV", name: "The Travelers Companies", sector: "Insurance" },
    TickerInfo { symbol: "COF", name: "Capital One Financial", sector: "Credit Services" },

    // Healthcare
    TickerInfo { symbol: "LLY", name: "Eli Lilly and Company", sector: "Pharmaceuticals" },
    TickerInfo { symbol: "UNH", name: "UnitedHealth Group", sector: "Healthcare Plans" },
    TickerInfo { symbol: "JNJ", name: "Johnson & Johnson", sector: "Healthcare" },
    TickerInfo { symbol: "ABBV", name: "AbbVie Inc.", sector: "Biotechnology" },
    TickerInfo { symbol: "MRK", name: "Merck & Co., Inc.", sector: "Pharmaceuticals" },
    TickerInfo { symbol: "TMO", name: "Thermo Fisher Scientific", sector: "Diagnostics / Tools" },
    TickerInfo { symbol: "ABT", name: "Abbott Laboratories", sector: "Medical Devices" },
    TickerInfo { symbol: "DHR", name: "Danaher Corporation", sector: "Diagnostics / Tools" },
    TickerInfo { symbol: "PFE", name: "Pfizer Inc.", sector: "Pharmaceuticals" },
    TickerInfo { symbol: "ISRG", name: "Intuitive Surgical", sector: "Medical Instruments" },
    TickerInfo { symbol: "AMGN", name: "Amgen Inc.", sector: "Biotechnology" },
    TickerInfo { symbol: "BMY", name: "Bristol Myers Squibb", sector: "Pharmaceuticals" },
    TickerInfo { symbol: "SYK", name: "Stryker Corporation", sector: "Medical Devices" },
    TickerInfo { symbol: "MDT", name: "Medtronic plc", sector: "Medical Devices" },
    TickerInfo { symbol: "GILD", name: "Gilead Sciences", sector: "Biotechnology" },
    TickerInfo { symbol: "VRTX", name: "Vertex Pharmaceuticals", sector: "Biotechnology" },
    TickerInfo { symbol: "REGN", name: "Regeneron Pharmaceuticals", sector: "Biotechnology" },
    TickerInfo { symbol: "CI", name: "The Cigna Group", sector: "Healthcare Plans" },
    TickerInfo { symbol: "CVS", name: "CVS Health Corporation", sector: "Healthcare Services" },
    TickerInfo { symbol: "ZTS", name: "Zoetis Inc.", sector: "Animal Health" },

    // Consumer Staples & Discretionary
    TickerInfo { symbol: "WMT", name: "Walmart Inc.", sector: "Consumer Retail" },
    TickerInfo { symbol: "COST", name: "Costco Wholesale Corp.", sector: "Discount Stores" },
    TickerInfo { symbol: "PG", name: "Procter & Gamble Co.", sector: "Household Products" },
    TickerInfo { symbol: "HD", name: "The Home Depot", sector: "Home Improvement" },
    TickerInfo { symbol: "MCD", name: "McDonald's Corporation", sector: "Restaurants" },
    TickerInfo { symbol: "KO", name: "The Coca-Cola Company", sector: "Beverages" },
    TickerInfo { symbol: "PEP", name: "PepsiCo, Inc.", sector: "Beverages" },
    TickerInfo { symbol: "NKE", name: "NIKE, Inc.", sector: "Footwear / Apparel" },
    TickerInfo { symbol: "PM", name: "Philip Morris International", sector: "Tobacco" },
    TickerInfo { symbol: "MO", name: "Altria Group, Inc.", sector: "Tobacco" },
    TickerInfo { symbol: "LOW", name: "Lowe's Companies", sector: "Home Improvement" },
    TickerInfo { symbol: "SBUX", name: "Starbucks Corporation", sector: "Restaurants" },
    TickerInfo { symbol: "TGT", name: "Target Corporation", sector: "Retail" },
    TickerInfo { symbol: "MDLZ", name: "Mondelez International", sector: "Confectionery" },
    TickerInfo { symbol: "CL", name: "Colgate-Palmolive", sector: "Personal Care" },
    TickerInfo { symbol: "TJX", name: "The TJX Companies", sector: "Apparel Retail" },
    TickerInfo { symbol: "BKNG", name: "Booking Holdings", sector: "Travel Services" },
    TickerInfo { symbol: "LMT", name: "Lockheed Martin", sector: "Aerospace & Defense" },
    TickerInfo { symbol: "BA", name: "The Boeing Company", sector: "Aerospace" },
    TickerInfo { symbol: "RTX", name: "RTX Corporation", sector: "Aerospace & Defense" },
    TickerInfo { symbol: "GE", name: "General Electric", sector: "Aerospace / Energy" },
    TickerInfo { symbol: "CAT", name: "Caterpillar Inc.", sector: "Heavy Machinery" },
    TickerInfo { symbol: "DE", name: "Deere & Company", sector: "Agricultural Machinery" },
    TickerInfo { symbol: "HON", name: "Honeywell International", sector: "Industrial Conglomerate" },
    TickerInfo { symbol: "UNP", name: "Union Pacific", sector: "Railroads" },
    TickerInfo { symbol: "UPS", name: "United Parcel Service", sector: "Logistics" },
    TickerInfo { symbol: "FDX", name: "FedEx Corporation", sector: "Logistics" },
    TickerInfo { symbol: "MMM", name: "3M Company", sector: "Industrial Conglomerate" },

    // Energy
    TickerInfo { symbol: "XOM", name: "Exxon Mobil Corporation", sector: "Oil & Gas" },
    TickerInfo { symbol: "CVX", name: "Chevron Corporation", sector: "Oil & Gas" },
    TickerInfo { symbol: "COP", name: "ConocoPhillips", sector: "Oil & Gas" },
    TickerInfo { symbol: "SLB", name: "SLB (Schlumberger)", sector: "Oilfield Services" },
    TickerInfo { symbol: "EOG", name: "EOG Resources", sector: "Oil & Gas" },
    TickerInfo { symbol: "OXY", name: "Occidental Petroleum", sector: "Oil & Gas" },
    TickerInfo { symbol: "MPC", name: "Marathon Petroleum", sector: "Refining" },
    TickerInfo { symbol: "VLO", name: "Valero Energy", sector: "Refining" },

    // Communication & Entertainment
    TickerInfo { symbol: "NFLX", name: "Netflix, Inc.", sector: "Entertainment" },
    TickerInfo { symbol: "DIS", name: "The Walt Disney Company", sector: "Entertainment" },
    TickerInfo { symbol: "CMCSA", name: "Comcast Corporation", sector: "Telecom / Media" },
    TickerInfo { symbol: "T", name: "AT&T Inc.", sector: "Telecom" },
    TickerInfo { symbol: "VZ", name: "Verizon Communications", sector: "Telecom" },
    TickerInfo { symbol: "TMUS", name: "T-Mobile US", sector: "Telecom" },
    TickerInfo { symbol: "SPOT", name: "Spotify Technology", sector: "Music Streaming" },

    // Utilities
    TickerInfo { symbol: "NEE", name: "NextEra Energy", sector: "Renewable Energy / Utilities" },
    TickerInfo { symbol: "SO", name: "The Southern Company", sector: "Electric Utilities" },
    TickerInfo { symbol: "DUK", name: "Duke Energy", sector: "Electric Utilities" },

    // Major ETFs
    TickerInfo { symbol: "SPY", name: "SPDR S&P 500 ETF Trust", sector: "Index ETF" },
    TickerInfo { symbol: "QQQ", name: "Invesco QQQ Trust", sector: "Tech ETF" },
    TickerInfo { symbol: "IWM", name: "iShares Russell 2000 ETF", sector: "Small-Cap ETF" },
    TickerInfo { symbol: "DIA", name: "SPDR Dow Jones Industrial ETF", sector: "Blue-Chip ETF" },
    TickerInfo { symbol: "VOO", name: "Vanguard S&P 500 ETF", sector: "Index ETF" },
    TickerInfo { symbol: "VTI", name: "Vanguard Total Stock Market", sector: "Broad Market ETF" },
    TickerInfo { symbol: "XLF", name: "Financial Select Sector SPDR", sector: "Financial ETF" },
    TickerInfo { symbol: "XLK", name: "Technology Select Sector SPDR", sector: "Tech ETF" },
    TickerInfo { symbol: "XLE", name: "Energy Select Sector SPDR", sector: "Energy ETF" },
    TickerInfo { symbol: "XLV", name: "Health Care Select Sector SPDR", sector: "Healthcare ETF" },
    TickerInfo { symbol: "SMH", name: "VanEck Semiconductor ETF", sector: "Semiconductor ETF" },
    TickerInfo { symbol: "GLD", name: "SPDR Gold Shares", sector: "Commodity ETF" },
    TickerInfo { symbol: "TLT", name: "iShares 20+ Year Treasury Bond", sector: "Bond ETF" },
];

/// Searches the ticker database with prioritization:
/// 1. Exact symbol match
/// 2. Symbol starts with query
/// 3. Symbol contains query
/// 4. Company name contains query
pub fn search_tickers(query: &str, limit: usize) -> Vec<&'static TickerInfo> {
    let q = query.trim().to_uppercase();
    if q.is_empty() {
        return Vec::new();
    }

    let mut exact = Vec::new();
    let mut prefix = Vec::new();
    let mut symbol_substr = Vec::new();
    let mut name_substr = Vec::new();

    for item in ALL_TICKERS {
        if item.symbol == q {
            exact.push(item);
        } else if item.symbol.starts_with(&q) {
            prefix.push(item);
        } else if item.symbol.contains(&q) {
            symbol_substr.push(item);
        } else if item.name.to_uppercase().contains(&q) {
            name_substr.push(item);
        }
    }

    let mut combined = Vec::with_capacity(limit);
    for item in exact.into_iter().chain(prefix).chain(symbol_substr).chain(name_substr) {
        if !combined.contains(&item) {
            combined.push(item);
            if combined.len() >= limit {
                break;
            }
        }
    }

    return combined;
}

/// Lookup company name by ticker symbol
pub fn lookup_name(symbol: &str) -> Option<&'static str> {
    let s = symbol.trim().to_uppercase();
    return ALL_TICKERS.iter().find(|t| t.symbol == s).map(|t| t.name);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_tickers_autocomplete() {
        let results = search_tickers("AA", 5);
        assert!(!results.is_empty());
        assert_eq!(results[0].symbol, "AAPL");

        let goog_results = search_tickers("GOOG", 5);
        assert!(goog_results.iter().any(|t| t.symbol == "GOOG"));
        assert!(goog_results.iter().any(|t| t.symbol == "GOOGL"));

        let apple_name = lookup_name("AAPL");
        assert_eq!(apple_name, Some("Apple Inc."));
    }
}
