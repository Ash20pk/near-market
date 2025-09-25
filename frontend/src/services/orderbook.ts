/**
 * Orderbook Service Integration
 * Connects to the NEAR Intent-based Prediction Marketplace orderbook service
 */

export interface OrderbookConfig {
  apiUrl: string;
  wsUrl: string;
}

export interface PriceData {
  market_id: string;
  outcome: number;
  bid: number;
  ask: number;
  mid: number;
  last?: number;
  timestamp: string;
}

export interface OrderbookSnapshot {
  market_id: string;
  outcome: number;
  bids: PriceLevel[];
  asks: PriceLevel[];
}

export interface PriceLevel {
  price: number;
  size: number;
  order_count: number;
}

export interface Trade {
  trade_id: string;
  market_id: string;
  maker_account: string;
  taker_account: string;
  maker_side: 'Buy' | 'Sell';
  taker_side: 'Buy' | 'Sell';
  outcome: number;
  price: number;
  size: number;
  executed_at: string;
}

export interface OrderSubmission {
  market_id: string;
  condition_id: string;
  user_account: string;
  outcome: number;
  side: 'Buy' | 'Sell';
  order_type: 'Market' | 'Limit';
  price: number;
  size: number;
  expires_at?: string | null;
  solver_account: string;
}

export interface CollateralBalance {
  account_id: string;
  market_id: string;
  available_balance: number;
  reserved_balance: number;
  position_balance: number;
}

interface CacheEntry<T> {
  data: T;
  timestamp: number;
  attempts: number;
  nextRetryAt: number;
}

interface BackoffConfig {
  initialDelayMs: number;
  maxDelayMs: number;
  backoffFactor: number;
  maxAttempts: number;
}

export class OrderbookService {
  private config: OrderbookConfig;
  private cache = new Map<string, CacheEntry<any>>();
  private readonly backoffConfig: BackoffConfig = {
    initialDelayMs: 1000,      // Start with 1 second
    maxDelayMs: 30000,         // Max 30 seconds
    backoffFactor: 2,          // Double the delay each time
    maxAttempts: 5             // Give up after 5 attempts
  };
  private readonly cacheConfig = {
    priceDataTtlMs: 5000,      // Price data expires after 5 seconds
    orderbookTtlMs: 3000,      // Orderbook expires after 3 seconds
    healthTtlMs: 10000,        // Health check expires after 10 seconds
  };

  constructor(config: OrderbookConfig) {
    this.config = config;

    // Clean up old cache entries every minute
    setInterval(() => this.cleanupCache(), 60000);
  }

  private cleanupCache() {
    const now = Date.now();
    let cleaned = 0;

    for (const [key, entry] of this.cache.entries()) {
      // Remove entries older than 5 minutes
      if (now - entry.timestamp > 300000) {
        this.cache.delete(key);
        cleaned++;
      }
    }

    if (cleaned > 0) {
      console.log(`[OrderbookService] Cleaned up ${cleaned} old cache entries`);
    }
  }

  private getCacheKey(type: string, ...params: string[]): string {
    return `${type}:${params.join(':')}`;
  }

  private isBackedOff(cacheKey: string): boolean {
    const entry = this.cache.get(cacheKey);
    if (!entry) return false;

    return Date.now() < entry.nextRetryAt;
  }

  private calculateBackoff(attempts: number): number {
    const delay = Math.min(
      this.backoffConfig.initialDelayMs * Math.pow(this.backoffConfig.backoffFactor, attempts),
      this.backoffConfig.maxDelayMs
    );
    return Date.now() + delay;
  }

  private updateCacheEntry<T>(cacheKey: string, data: T | null, ttl: number, success: boolean) {
    const existingEntry = this.cache.get(cacheKey);
    const attempts = success ? 0 : (existingEntry?.attempts || 0) + 1;

    const entry: CacheEntry<T> = {
      data: data!,
      timestamp: Date.now(),
      attempts,
      nextRetryAt: success ? 0 : this.calculateBackoff(attempts)
    };

    this.cache.set(cacheKey, entry);

    if (!success && attempts < this.backoffConfig.maxAttempts) {
      const nextRetryIn = Math.round((entry.nextRetryAt - Date.now()) / 1000);
      console.log(`[OrderbookService] Backing off ${cacheKey} for ${nextRetryIn}s (attempt ${attempts}/${this.backoffConfig.maxAttempts})`);
    } else if (!success) {
      console.log(`[OrderbookService] Max attempts reached for ${cacheKey}, giving up until cache expires`);
    }
  }

  private getCachedData<T>(cacheKey: string, ttl: number): T | null {
    const entry = this.cache.get(cacheKey);
    if (!entry) return null;

    const now = Date.now();

    // If we're in backoff period, return cached data (even if stale)
    if (now < entry.nextRetryAt) {
      return entry.data;
    }

    // If data is fresh, return it
    if (now - entry.timestamp < ttl) {
      return entry.data;
    }

    // Data is stale
    return null;
  }

  /**
   * Check if a market likely has no trading data based on cached failures
   * This allows callers to skip expensive calls for markets known to have no data
   */
  hasKnownNoTradingData(marketId: string): boolean {
    const priceKey = this.getCacheKey('price', marketId, '1');
    const orderbookKey = this.getCacheKey('orderbook', marketId, '1');

    const priceEntry = this.cache.get(priceKey);
    const orderbookEntry = this.cache.get(orderbookKey);

    // If both price and orderbook have been tried and failed (404s), and we're in backoff,
    // it's likely this market has no trading data
    const priceBackedOff = priceEntry && Date.now() < priceEntry.nextRetryAt && priceEntry.data === null;
    const orderbookBackedOff = orderbookEntry && Date.now() < orderbookEntry.nextRetryAt && orderbookEntry.data === null;

    return Boolean(priceBackedOff && orderbookBackedOff);
  }

  /**
   * Convert price from frontend format (0-100) to orderbook format (0-100000)
   */
  static priceToOrderbook(frontendPrice: number): number {
    return Math.round(frontendPrice * 1000);
  }

  /**
   * Convert price from orderbook format (0-100000) to frontend format (0-100)
   */
  static priceFromOrderbook(orderbookPrice: number): number {
    return orderbookPrice / 1000;
  }

  /**
   * Convert size from frontend format to micro-tokens
   */
  static sizeToOrderbook(frontendSize: number): number {
    return Math.round(frontendSize * 1_000_000);
  }

  /**
   * Convert size from micro-tokens to frontend format
   */
  static sizeFromOrderbook(orderbookSize: number): number {
    return orderbookSize / 1_000_000;
  }

  /**
   * Check if orderbook service is healthy
   */
  async checkHealth(): Promise<boolean> {
    const cacheKey = this.getCacheKey('health');

    // Check cache first
    const cachedData = this.getCachedData<boolean>(cacheKey, this.cacheConfig.healthTtlMs);
    if (cachedData !== null) {
      console.log(`[OrderbookService] Using cached health status: ${cachedData}`);
      return cachedData;
    }

    // Check if we're in backoff period
    if (this.isBackedOff(cacheKey)) {
      console.log(`[OrderbookService] Skipping health check - in backoff period`);
      return false; // Assume unhealthy if we can't check
    }

    console.log(`[OrderbookService] Checking orderbook service health...`);

    try {
      const response = await fetch(`${this.config.apiUrl}/health`);
      const isHealthy = response.ok;

      // Cache the result
      this.updateCacheEntry(cacheKey, isHealthy, this.cacheConfig.healthTtlMs, true);
      console.log(`[OrderbookService] Health check result: ${isHealthy ? 'healthy' : 'unhealthy'}`);

      return isHealthy;
    } catch (error) {
      console.error('[OrderbookService] Health check failed:', error);

      // Cache the failure
      this.updateCacheEntry(cacheKey, false, this.cacheConfig.healthTtlMs, false);
      return false;
    }
  }

  /**
   * Get current market price for a specific outcome
   */
  async getMarketPrice(marketId: string, outcome: number): Promise<PriceData | null> {
    const cacheKey = this.getCacheKey('price', marketId, outcome.toString());

    // Check cache first
    const cachedData = this.getCachedData<PriceData>(cacheKey, this.cacheConfig.priceDataTtlMs);
    if (cachedData !== null) {
      console.log(`[OrderbookService] Using cached price data for ${marketId}:${outcome}`);
      return cachedData;
    }

    // Check if we're in backoff period
    if (this.isBackedOff(cacheKey)) {
      console.log(`[OrderbookService] Skipping price request for ${marketId}:${outcome} - in backoff period`);
      return null;
    }

    console.log(`[OrderbookService] Fetching fresh price data for ${marketId}:${outcome}`);

    try {
      const url = `${this.config.apiUrl}/price/${marketId}/${outcome}`;
      const response = await fetch(url);

      if (!response.ok) {
        if (response.status === 404) {
          console.log(`[OrderbookService] No price data for ${marketId}:${outcome} (normal for new markets)`);
          this.updateCacheEntry(cacheKey, null, this.cacheConfig.priceDataTtlMs, true);
          return null;
        }
        throw new Error(`HTTP ${response.status}`);
      }

      const data = await response.json();

      // Convert prices from orderbook format to frontend format
      const convertedPrice = {
        ...data,
        bid: OrderbookService.priceFromOrderbook(data.bid),
        ask: OrderbookService.priceFromOrderbook(data.ask),
        mid: OrderbookService.priceFromOrderbook(data.mid),
        last: data.last ? OrderbookService.priceFromOrderbook(data.last) : undefined,
      };

      // Cache successful response
      this.updateCacheEntry(cacheKey, convertedPrice, this.cacheConfig.priceDataTtlMs, true);
      console.log(`[OrderbookService] ✅ Successfully fetched and cached price data for ${marketId}:${outcome}`);

      return convertedPrice;
    } catch (error) {
      console.error(`[OrderbookService] ❌ Failed to get price for ${marketId}:${outcome}:`, error);

      // Cache the failure to trigger backoff
      this.updateCacheEntry(cacheKey, null, this.cacheConfig.priceDataTtlMs, false);
      return null;
    }
  }

  /**
   * Get orderbook snapshot for a market
   */
  async getOrderbook(marketId: string, outcome: number): Promise<OrderbookSnapshot | null> {
    const cacheKey = this.getCacheKey('orderbook', marketId, outcome.toString());

    // Check cache first
    const cachedData = this.getCachedData<OrderbookSnapshot>(cacheKey, this.cacheConfig.orderbookTtlMs);
    if (cachedData !== null) {
      console.log(`[OrderbookService] Using cached orderbook data for ${marketId}:${outcome}`);
      return cachedData;
    }

    // Check if we're in backoff period
    if (this.isBackedOff(cacheKey)) {
      console.log(`[OrderbookService] Skipping orderbook request for ${marketId}:${outcome} - in backoff period`);
      return null;
    }

    console.log(`[OrderbookService] Fetching fresh orderbook data for ${marketId}:${outcome}`);

    try {
      const response = await fetch(`${this.config.apiUrl}/orderbook/${marketId}/${outcome}`);
      if (!response.ok) {
        if (response.status === 404) {
          console.log(`[OrderbookService] No orderbook data for ${marketId}:${outcome} (normal for new markets)`);
          this.updateCacheEntry(cacheKey, null, this.cacheConfig.orderbookTtlMs, true);
          return null;
        }
        throw new Error(`HTTP ${response.status}`);
      }

      const data = await response.json();

      // Convert prices and sizes from orderbook format
      const convertedOrderbook = {
        ...data,
        bids: data.bids.map((level: any) => ({
          ...level,
          price: OrderbookService.priceFromOrderbook(level.price),
          size: OrderbookService.sizeFromOrderbook(level.size),
        })),
        asks: data.asks.map((level: any) => ({
          ...level,
          price: OrderbookService.priceFromOrderbook(level.price),
          size: OrderbookService.sizeFromOrderbook(level.size),
        })),
      };

      // Cache successful response
      this.updateCacheEntry(cacheKey, convertedOrderbook, this.cacheConfig.orderbookTtlMs, true);
      console.log(`[OrderbookService] ✅ Successfully fetched and cached orderbook for ${marketId}:${outcome}`);

      return convertedOrderbook;
    } catch (error) {
      console.error(`[OrderbookService] ❌ Failed to get orderbook for ${marketId}:${outcome}:`, error);

      // Cache the failure to trigger backoff
      this.updateCacheEntry(cacheKey, null, this.cacheConfig.orderbookTtlMs, false);
      return null;
    }
  }

  /**
   * Submit an order to the orderbook
   */
  async submitOrder(order: OrderSubmission): Promise<{success: boolean, orderId?: string, error?: string}> {
    try {
      const orderbookOrder = {
        ...order,
        price: OrderbookService.priceToOrderbook(order.price),
        size: OrderbookService.sizeToOrderbook(order.size),
      };

      const response = await fetch(`${this.config.apiUrl}/orders`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(orderbookOrder),
      });

      const result = await response.json();

      if (response.ok) {
        return {
          success: true,
          orderId: result.order_id,
        };
      } else {
        return {
          success: false,
          error: result.error || 'Order submission failed',
        };
      }
    } catch (error) {
      console.error('Failed to submit order:', error);
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }
  }

  /**
   * Get collateral balance for a user in a specific market
   */
  async getCollateralBalance(accountId: string, marketId: string): Promise<CollateralBalance | null> {
    try {
      const response = await fetch(`${this.config.apiUrl}/collateral/balance`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          account_id: accountId,
          market_id: marketId,
        }),
      });

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`);
      }

      const data = await response.json();

      // Convert from micro-USDC to USDC
      return {
        ...data,
        available_balance: data.available_balance / 1_000_000,
        reserved_balance: data.reserved_balance / 1_000_000,
        position_balance: data.position_balance / 1_000_000,
      };
    } catch (error) {
      console.error('Failed to get collateral balance:', error);
      return null;
    }
  }

  /**
   * Get market liquidity data (for display purposes)
   */
  async getMarketLiquidity(marketId: string, outcome: number): Promise<{asks: PriceLevel[], bids: PriceLevel[]} | null> {
    try {
      const response = await fetch(`${this.config.apiUrl}/solver/liquidity/${marketId}/${outcome}`);
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`);
      }

      const data = await response.json();

      // Convert prices and sizes from orderbook format
      return {
        asks: data.asks.map((level: any) => ({
          price: OrderbookService.priceFromOrderbook(parseInt(level.price)),
          size: OrderbookService.sizeFromOrderbook(parseInt(level.size)),
          order_count: level.orders,
        })),
        bids: data.bids.map((level: any) => ({
          price: OrderbookService.priceFromOrderbook(parseInt(level.price)),
          size: OrderbookService.sizeFromOrderbook(parseInt(level.size)),
          order_count: level.orders,
        })),
      };
    } catch (error) {
      console.error('Failed to get market liquidity:', error);
      return null;
    }
  }
}

/**
 * Default configuration for local development
 */
export const defaultOrderbookConfig: OrderbookConfig = {
  apiUrl: process.env.NEXT_PUBLIC_ORDERBOOK_API_URL || 'http://localhost:8080',
  wsUrl: process.env.NEXT_PUBLIC_ORDERBOOK_WS_URL || 'ws://localhost:8080/ws',
};

/**
 * Global orderbook service instance
 */
export const orderbookService = new OrderbookService(defaultOrderbookConfig);