/**
 * Orderbook Configuration
 * Configuration settings for orderbook integration
 */

export const orderbookConfig = {
  // API endpoints
  api: {
    url: process.env.NEXT_PUBLIC_ORDERBOOK_API_URL || 'http://localhost:8080',
    wsUrl: process.env.NEXT_PUBLIC_ORDERBOOK_WS_URL || 'ws://localhost:8080/ws',
  },

  // WebSocket settings
  websocket: {
    reconnectAttempts: 5,
    reconnectDelay: 1000,
    maxReconnectDelay: 30000,
    pingInterval: 30000,
  },

  // UI settings
  ui: {
    defaultPrecision: 1,
    maxOrderbookLevels: 10,
    maxTradeHistory: 50,
    priceUpdateInterval: 30000, // 30 seconds
    balanceUpdateInterval: 60000, // 1 minute
  },

  // Trading settings
  trading: {
    defaultOrderType: 'Limit' as const,
    minPrice: 0.1,
    maxPrice: 99.9,
    priceStep: 0.1,
    minSize: 1,
    sizeStep: 1,
    quickTradeSizes: [10, 100, 1000],
  },

  // Format settings
  format: {
    currency: '¢',
    thousandsSeparator: ',',
    decimalPlaces: 1,
    compactThreshold: 1000,
  },

  // Features flags
  features: {
    realTimeUpdates: true,
    orderSubmission: true,
    balanceChecking: true,
    tradeHistory: true,
    marketPricing: true,
  },
};

/**
 * Environment-specific configurations
 */
export const environments = {
  development: {
    ...orderbookConfig,
    api: {
      url: 'http://localhost:8080',
      wsUrl: 'ws://localhost:8080/ws',
    },
  },

  staging: {
    ...orderbookConfig,
    api: {
      url: 'https://staging-orderbook.yourapp.com',
      wsUrl: 'wss://staging-orderbook.yourapp.com/ws',
    },
  },

  production: {
    ...orderbookConfig,
    api: {
      url: 'https://orderbook.yourapp.com',
      wsUrl: 'wss://orderbook.yourapp.com/ws',
    },
    websocket: {
      ...orderbookConfig.websocket,
      reconnectAttempts: 10,
    },
  },
};

/**
 * Get configuration for current environment
 */
export function getOrderbookConfig() {
  const env = process.env.NODE_ENV || 'development';
  return environments[env as keyof typeof environments] || environments.development;
}

/**
 * Utility functions for configuration
 */
export const configUtils = {
  /**
   * Format price according to config
   */
  formatPrice: (price: number) => {
    const { currency, decimalPlaces } = orderbookConfig.format;
    return `${price.toFixed(decimalPlaces)}${currency}`;
  },

  /**
   * Format size with compact notation
   */
  formatSize: (size: number) => {
    const { compactThreshold, thousandsSeparator } = orderbookConfig.format;

    if (size >= compactThreshold) {
      if (size >= 1000000) return `${(size / 1000000).toFixed(1)}M`;
      if (size >= 1000) return `${(size / 1000).toFixed(1)}K`;
    }

    return size.toLocaleString('en-US', {
      useGrouping: true,
    });
  },

  /**
   * Validate price within bounds
   */
  validatePrice: (price: number) => {
    const { minPrice, maxPrice, priceStep } = orderbookConfig.trading;

    if (price < minPrice || price > maxPrice) {
      return false;
    }

    // Check if price aligns with step
    const remainder = (price * 10) % (priceStep * 10);
    return Math.abs(remainder) < 0.01;
  },

  /**
   * Validate size
   */
  validateSize: (size: number) => {
    const { minSize, sizeStep } = orderbookConfig.trading;

    if (size < minSize) {
      return false;
    }

    // Check if size aligns with step
    const remainder = size % sizeStep;
    return Math.abs(remainder) < 0.01;
  },

  /**
   * Round price to valid increment
   */
  roundPrice: (price: number) => {
    const { priceStep } = orderbookConfig.trading;
    return Math.round(price / priceStep) * priceStep;
  },

  /**
   * Round size to valid increment
   */
  roundSize: (size: number) => {
    const { sizeStep } = orderbookConfig.trading;
    return Math.round(size / sizeStep) * sizeStep;
  },
};

export default orderbookConfig;