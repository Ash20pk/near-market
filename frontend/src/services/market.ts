/**
 * Market Service - Combines data from NEAR contracts and orderbook service
 * Provides unified market data with live trading information
 */

import { Market } from '@/lib/near';
import { OrderbookService, defaultOrderbookConfig } from './orderbook';

export interface EnhancedMarket extends Market {
  // Live trading data from orderbook
  current_price?: number; // Current mid price
  price_change_24h?: number; // 24h price change percentage
  liquidity?: number; // Total orderbook liquidity
  trades_24h?: number; // Number of trades in 24h
  best_bid?: number;
  best_ask?: number;
  spread?: number;
}

export class MarketService {
  private orderbookService: OrderbookService;

  constructor() {
    this.orderbookService = new OrderbookService(defaultOrderbookConfig);
  }

  /**
   * Get all markets with live trading data
   */
  async getMarkets(category?: string, isActive?: boolean): Promise<EnhancedMarket[]> {
    console.log('[MarketService] getMarkets called with filters:', { category, isActive });

    try {
      // Get markets from NEAR only
      console.log('[MarketService] Fetching markets from sources...');
      const [nearMarkets] = await Promise.allSettled([
        this.getNearMarkets(category, isActive),
      ]);

      let markets: Market[] = [];

      // Only use NEAR markets
      if (nearMarkets.status === 'fulfilled' && nearMarkets.value.length > 0) {
        markets = nearMarkets.value;
        console.log('[MarketService] Found', markets.length, 'markets from NEAR');
      } else if (nearMarkets.status === 'rejected') {
        console.error('[MarketService] NEAR markets fetch failed:', nearMarkets.reason);
      } else {
        console.log('[MarketService] No markets found in NEAR');
      }

      if (markets.length === 0) {
        console.log('[MarketService] ⚠️ No markets available from any source');
        console.log('[MarketService] Troubleshooting:');
        console.log('[MarketService]   1. Check if user is signed in to NEAR wallet');
        console.log('[MarketService]   2. Verify contract address: verifier.ashpk20.testnet');
        console.log('[MarketService]   3. Ensure markets exist in the deployed contract');
        console.log('[MarketService]   4. Check network connectivity to NEAR RPC');
        return [];
      }

      // Enhance each market with live trading data from orderbook
      console.log('[MarketService] Enhancing', markets.length, 'markets with orderbook data...');

      // Apply basic filters first to avoid unnecessary orderbook calls
      const filteredMarkets = markets.filter(market => {
        if (category && market.category !== category) return false;
        if (isActive !== undefined && market.is_active !== isActive) return false;
        return true;
      });

      console.log('[MarketService] After filtering:', filteredMarkets.length, 'markets to enhance');

      if (filteredMarkets.length === 0) {
        console.log('[MarketService] No markets match the filters, returning empty array');
        return [];
      }

      // Try to enhance markets with orderbook data
      const enhancedMarkets = await Promise.allSettled(
        filteredMarkets.map(market => this.enhanceMarketWithTradingData(market))
      );

      const validMarkets = enhancedMarkets
        .filter((result): result is PromisedSettledResult<EnhancedMarket> & { status: 'fulfilled' } =>
          result.status === 'fulfilled')
        .map(result => result.value);

      // If enhancement failed for all markets, fallback to basic NEAR markets
      if (validMarkets.length === 0 && filteredMarkets.length > 0) {
        console.log('[MarketService] ⚠️ All market enhancements failed, falling back to basic NEAR markets');
        return filteredMarkets.map(market => ({ ...market } as EnhancedMarket));
      }

      console.log('[MarketService] Successfully returning', validMarkets.length, 'enhanced markets');
      return validMarkets;

    } catch (error) {
      console.error('[MarketService] Error fetching markets:', error);
      return [];
    }
  }

  /**
   * Get single market with live trading data
   */
  async getMarket(marketId: string): Promise<EnhancedMarket | null> {
    console.log(`[MarketService] 📊 Getting single market: ${marketId}`);

    try {
      // Strategy 1: Try to get directly from NEAR
      let market = await this.getNearMarket(marketId);

      // Strategy 2: If direct fetch fails, get from markets list and find by ID
      if (!market) {
        console.log('[MarketService] Direct market fetch failed, trying from markets list...');
        const allMarkets = await this.getNearMarkets();
        market = allMarkets.find(m => m.market_id === marketId) || null;

        if (market) {
          console.log('[MarketService] ✅ Found market in markets list:', market.title);
        } else {
          console.log('[MarketService] ❌ Market not found in markets list either');
        }
      }

      // Strategy 3: No orderbook fallback anymore - only real NEAR markets
      if (!market) {
        console.log(`[MarketService] ❌ Market ${marketId} not found anywhere`);
        return null;
      }

      // Enhance with trading data
      console.log(`[MarketService] 🚀 Enhancing market ${marketId} with trading data...`);
      return await this.enhanceMarketWithTradingData(market);
    } catch (error) {
      console.error(`[MarketService] ❌ Error fetching market ${marketId}:`, error);
      return null;
    }
  }

  /**
   * Get markets from NEAR verifier contract
   */
  private async getNearMarkets(category?: string, isActive?: boolean): Promise<Market[]> {
    console.log('[MarketService] 🔍 Attempting to fetch markets from NEAR contract');
    console.log('[MarketService] Expected contract address: verifier.ashpk20.testnet');
    console.log('[MarketService] Filters:', { category, isActive });

    try {
      // Import the near service properly
      const { nearService } = await import('@/lib/near');

      if (!nearService) {
        console.log('[MarketService] ❌ NEAR service not initialized');
        return [];
      }

      console.log('[MarketService] ✅ NEAR service available, checking access mode...');
      console.log('[MarketService] User signed in:', nearService.isSignedIn());
      console.log('[MarketService] Access mode:', nearService.isSignedIn() ? 'Full contract access' : 'View-only contract access');
      console.log('[MarketService] Note: View methods work for both signed and unsigned users');

      console.log('[MarketService] 📡 Calling nearService.getMarkets with filters:', { category, isActive });
      const markets = await nearService.getMarkets(category, isActive);

      console.log('[MarketService] 📊 Retrieved', markets?.length || 0, 'markets from NEAR contract');

      if (markets && markets.length > 0) {
        console.log('[MarketService] ✅ First market found:', markets[0].market_id);
        console.log('[MarketService] Market details:', {
          title: markets[0].title,
          creator: markets[0].creator,
          category: markets[0].category,
          is_active: markets[0].is_active
        });
      } else {
        console.log('[MarketService] ℹ️ No markets found in NEAR contract. This could mean:');
        console.log('[MarketService]   - No markets have been created yet');
        console.log('[MarketService]   - Markets exist but don\'t match the filters');
        console.log('[MarketService]   - Contract connection issue');
      }

      return markets || [];
    } catch (error) {
      console.error('[MarketService] ❌ Error fetching NEAR markets:', error);
      console.error('[MarketService] Error details:', {
        name: error instanceof Error ? error.name : 'Unknown',
        message: error instanceof Error ? error.message : String(error),
        stack: error instanceof Error ? error.stack : undefined
      });
      return [];
    }
  }

  /**
   * Get single market from NEAR
   */
  private async getNearMarket(marketId: string): Promise<Market | null> {
    console.log('[MarketService] 🔍 Fetching single market from NEAR:', marketId);

    try {
      const { nearService } = await import('@/lib/near');

      if (!nearService) {
        console.log('[MarketService] ❌ NEAR service not initialized');
        return null;
      }

      console.log('[MarketService] ✅ NEAR service available, calling getMarket...');
      console.log('[MarketService] User signed in:', nearService.isSignedIn());

      // getMarket should work for both signed and unsigned users (view methods)
      const market = await nearService.getMarket(marketId);

      if (market) {
        console.log('[MarketService] ✅ Successfully fetched market from NEAR:', market.title);
      } else {
        console.log('[MarketService] ❌ Market not found in NEAR contract:', marketId);
      }

      return market;
    } catch (error) {
      console.error('[MarketService] ❌ Error fetching NEAR market:', error);
      console.error('[MarketService] Error details:', {
        name: error instanceof Error ? error.name : 'Unknown',
        message: error instanceof Error ? error.message : String(error)
      });
      return null;
    }
  }

  /**
   * Get markets that exist in the orderbook service - DISABLED
   * Only using real NEAR markets now
   */
  private async getOrderbookMarkets(): Promise<Market[]> {
    console.log('[MarketService] Skipping orderbook market discovery - only using NEAR markets');
    return []; // Only use real NEAR markets
  }

  /**
   * Enhance market with live trading data from orderbook
   */
  private async enhanceMarketWithTradingData(market: Market): Promise<EnhancedMarket> {
    const enhancedMarket: EnhancedMarket = { ...market };

    try {
      console.log(`[MarketService] Attempting to enhance market ${market.market_id} with orderbook data...`);

      // Check if we already know this market has no trading data
      if (this.orderbookService.hasKnownNoTradingData(market.market_id)) {
        console.log(`[MarketService] ℹ️ Skipping orderbook calls for market ${market.market_id} - cached failures indicate no trading data yet`);
        return enhancedMarket;
      }

      // Set reasonable timeout for orderbook calls (2 seconds)
      const timeoutPromise = new Promise((_, reject) =>
        setTimeout(() => reject(new Error('Orderbook timeout')), 2000)
      );

      // Get live price data for both outcomes with timeout
      const [outcome1Price, outcome0Price, orderbook] = await Promise.allSettled([
        Promise.race([this.orderbookService.getMarketPrice(market.market_id, 1), timeoutPromise]),
        Promise.race([this.orderbookService.getMarketPrice(market.market_id, 0), timeoutPromise]),
        Promise.race([this.orderbookService.getOrderbook(market.market_id, 1), timeoutPromise]),
      ]);

      let hasAnyTradingData = false;

      // Process outcome 1 (YES) price data
      if (outcome1Price.status === 'fulfilled' && outcome1Price.value) {
        const price = outcome1Price.value;
        if (price.mid !== undefined && price.mid !== null) {
          enhancedMarket.current_price = price.mid;
          hasAnyTradingData = true;
        }
        if (price.bid !== undefined && price.bid !== null) {
          enhancedMarket.best_bid = price.bid;
        }
        if (price.ask !== undefined && price.ask !== null) {
          enhancedMarket.best_ask = price.ask;
        }

        if (enhancedMarket.best_bid && enhancedMarket.best_ask) {
          enhancedMarket.spread = enhancedMarket.best_ask - enhancedMarket.best_bid;
        }

        // Calculate price change if we have last price
        if (price.last && price.mid && price.last !== price.mid) {
          enhancedMarket.price_change_24h = ((price.mid - price.last) / price.last) * 100;
        }
      } else if (outcome1Price.status === 'rejected') {
        console.log(`[MarketService] Could not get price data for market ${market.market_id}: ${outcome1Price.reason}`);
      }

      // Process orderbook data for liquidity
      if (orderbook.status === 'fulfilled' && orderbook.value && orderbook.value.bids && orderbook.value.asks) {
        const book = orderbook.value;
        const bidLiquidity = book.bids.reduce((sum, level) => sum + level.size, 0);
        const askLiquidity = book.asks.reduce((sum, level) => sum + level.size, 0);
        const totalLiquidity = bidLiquidity + askLiquidity;

        if (totalLiquidity > 0) {
          enhancedMarket.liquidity = totalLiquidity;
          hasAnyTradingData = true;
        }
      } else if (orderbook.status === 'rejected') {
        console.log(`[MarketService] Could not get orderbook data for market ${market.market_id}: ${orderbook.reason}`);
      }

      if (hasAnyTradingData) {
        console.log(`[MarketService] ✅ Enhanced market ${market.market_id} with trading data`);
      } else {
        console.log(`[MarketService] ℹ️ No trading data available for market ${market.market_id} (this is normal for new untested markets)`);
      }

      // trades_24h will be calculated from actual trade history when available

    } catch (error) {
      // Gracefully handle any enhancement failures
      console.log(`[MarketService] ⚠️ Could not enhance market ${market.market_id} with trading data: ${error instanceof Error ? error.message : String(error)}`);
    }

    return enhancedMarket;
  }

  /**
   * Generate market title from market ID
   */
  private generateMarketTitle(marketId: string): string {
    if (marketId.includes('ashpk20.testnet')) {
      const timestamp = marketId.split('_')[1];
      return `Test Market ${timestamp.slice(-6)} - Binary Prediction`;
    }
    if (marketId === 'market_1') return 'Test Market #1 - Sample Prediction';
    if (marketId === 'market_2') return 'Test Market #2 - Binary Outcome';
    if (marketId === 'btc-100k-2024') return 'Will Bitcoin reach $100,000 by end of 2024?';
    if (marketId === 'eth-5k-2024') return 'Will Ethereum reach $5,000 by end of 2024?';

    return `Market ${marketId} - Binary Prediction`;
  }

  /**
   * Generate market description from market ID
   */
  private generateMarketDescription(marketId: string): string {
    if (marketId.includes('ashpk20.testnet')) {
      return 'This is a test market created for orderbook integration testing. It will resolve based on test conditions.';
    }
    if (marketId.startsWith('market_')) {
      return 'A test market for demonstrating the prediction marketplace functionality with live orderbook integration.';
    }
    if (marketId === 'btc-100k-2024') {
      return 'This market resolves to "Yes" if Bitcoin (BTC) reaches or exceeds $100,000 USD on any major exchange by December 31, 2024, 11:59 PM UTC.';
    }
    if (marketId === 'eth-5k-2024') {
      return 'This market resolves to "Yes" if Ethereum (ETH) reaches or exceeds $5,000 USD on any major exchange by December 31, 2024.';
    }

    return 'A prediction market with binary outcomes (YES/NO) integrated with live orderbook trading.';
  }

  /**
   * Extract creator from market ID
   */
  private extractCreatorFromMarketId(marketId: string): string {
    if (marketId.includes('ashpk20.testnet')) return 'ashpk20.testnet';
    return 'testuser.testnet';
  }

  /**
   * Categorize market by ID
   */
  private categorizeMarket(marketId: string): string {
    if (marketId.includes('btc') || marketId.includes('eth') || marketId.includes('crypto')) return 'Crypto';
    if (marketId.includes('ashpk20.testnet') || marketId.startsWith('market_')) return 'Technology';
    return 'Other';
  }

}

// Export singleton instance
export const marketService = new MarketService();