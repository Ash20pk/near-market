/**
 * React Hooks for Orderbook Integration
 * Easy-to-use hooks for connecting frontend components to the orderbook service
 */

import { useState, useEffect, useCallback, useRef } from 'react';
import {
  OrderbookService,
  OrderbookSnapshot,
  PriceData,
  Trade,
  CollateralBalance,
  OrderSubmission,
  defaultOrderbookConfig,
  orderbookService // Import the global shared instance
} from '../services/orderbook';
import { useGlobalWebSocket } from './useGlobalWebSocket';

/**
 * Hook for managing orderbook connection and real-time data
 */
export function useOrderbook(marketId?: string, outcome?: number) {
  const [orderbook, setOrderbook] = useState<OrderbookSnapshot | null>(null);
  const [recentTrades, setRecentTrades] = useState<Trade[]>([]);
  const [price, setPrice] = useState<PriceData | null>(null);

  // Use the global shared orderbook service instance instead of creating new ones
  const orderbookServiceRef = useRef(orderbookService);

  // Use global WebSocket connection (persistent across page changes)
  const { isConnected, error } = useGlobalWebSocket({
    marketId,
    outcome,
    onOrderbookUpdate: (message) => {
      setOrderbook(message.snapshot);
    },
    onTradeExecuted: (message) => {
      setRecentTrades(prev => [message.trade, ...prev.slice(0, 49)]);
    },
    onOrderUpdate: (message) => {
      console.log('Order updated:', message);
    },
  });

  // Fetch initial data when market/outcome changes
  useEffect(() => {
    if (marketId && outcome !== undefined) {
      fetchMarketData();
    }
  }, [marketId, outcome]);

  const fetchMarketData = useCallback(async () => {
    if (!marketId || outcome === undefined) return;

    try {
      // Fetch price and orderbook data
      const [priceData, orderbookData] = await Promise.allSettled([
        orderbookServiceRef.current.getMarketPrice(marketId, outcome),
        orderbookServiceRef.current.getOrderbook(marketId, outcome),
      ]);

      // Handle price data result
      if (priceData.status === 'fulfilled' && priceData.value) {
        setPrice(priceData.value);
      } else if (priceData.status === 'rejected' && !priceData.reason?.message?.includes('HTTP 404')) {
        console.error('Failed to fetch market price:', priceData.reason);
      }

      // Handle orderbook data result
      if (orderbookData.status === 'fulfilled' && orderbookData.value) {
        setOrderbook(orderbookData.value);
      } else if (orderbookData.status === 'rejected' && !orderbookData.reason?.message?.includes('HTTP 404')) {
        console.error('Failed to fetch orderbook data:', orderbookData.reason);
      }

      // Only set error if both failed for non-404 reasons
      if (priceData.status === 'rejected' && orderbookData.status === 'rejected' &&
          !priceData.reason?.message?.includes('HTTP 404') &&
          !orderbookData.reason?.message?.includes('HTTP 404')) {
        setError('Failed to fetch market data');
      }
    } catch (error) {
      console.error('Failed to fetch market data:', error);
      setError('Failed to fetch market data');
    }
  }, [marketId, outcome]);

  const submitOrder = useCallback(async (order: OrderSubmission) => {
    return await orderbookServiceRef.current.submitOrder(order);
  }, []);

  const getCollateralBalance = useCallback(async (accountId: string, marketId: string) => {
    return await orderbookServiceRef.current.getCollateralBalance(accountId, marketId);
  }, []);

  const checkHealth = useCallback(async () => {
    return await orderbookServiceRef.current.checkHealth();
  }, []);

  return {
    // Connection status
    isConnected,
    error,

    // Market data
    orderbook,
    price,
    recentTrades,

    // Actions
    submitOrder,
    getCollateralBalance,
    checkHealth,
    refetch: fetchMarketData,
  };
}

/**
 * Hook for managing user's collateral balance
 */
export function useCollateralBalance(accountId?: string, marketId?: string) {
  const [balance, setBalance] = useState<CollateralBalance | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Use the global shared orderbook service instance instead of creating new ones
  const orderbookServiceRef = useRef(orderbookService);

  const fetchBalance = useCallback(async () => {
    if (!accountId || !marketId) return;

    setLoading(true);
    setError(null);

    try {
      const balanceData = await orderbookServiceRef.current.getCollateralBalance(accountId, marketId);
      setBalance(balanceData);
    } catch (error) {
      console.error('Failed to fetch collateral balance:', error);
      setError('Failed to fetch balance');
    } finally {
      setLoading(false);
    }
  }, [accountId, marketId]);

  useEffect(() => {
    if (accountId && marketId) {
      fetchBalance();
    }
  }, [accountId, marketId, fetchBalance]);

  return {
    balance,
    loading,
    error,
    refetch: fetchBalance,
  };
}

/**
 * Hook for market price data with automatic updates
 */
export function useMarketPrice(marketId?: string, outcome?: number, pollInterval = 60000) {
  const [price, setPrice] = useState<PriceData | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Use the global shared orderbook service instance instead of creating new ones
  const orderbookServiceRef = useRef(orderbookService);

  const fetchPrice = useCallback(async () => {
    if (!marketId || outcome === undefined) return;

    setLoading(true);
    setError(null);

    try {
      const priceData = await orderbookServiceRef.current.getMarketPrice(marketId, outcome);
      if (priceData) {
        setPrice(priceData);
      }
    } catch (error) {
      // Don't treat 404 as an error - market just doesn't exist in orderbook yet
      if (error instanceof Error && error.message.includes('HTTP 404')) {
        // Silently fail for 404 - this allows fallback components to work
        console.log(`Market ${marketId} not found in orderbook - using fallback`);
      } else {
        console.error('Failed to fetch market price:', error);
        setError('Failed to fetch price');
      }
    } finally {
      setLoading(false);
    }
  }, [marketId, outcome]);

  useEffect(() => {
    if (marketId && outcome !== undefined) {
      fetchPrice();

      // Set up polling interval
      const interval = setInterval(fetchPrice, pollInterval);
      return () => clearInterval(interval);
    }
  }, [marketId, outcome, pollInterval, fetchPrice]);

  return {
    price,
    loading,
    error,
    refetch: fetchPrice,
  };
}

/**
 * Hook for submitting orders with state management
 */
export function useOrderSubmission() {
  const [submitting, setSubmitting] = useState(false);
  const [lastResult, setLastResult] = useState<{success: boolean, orderId?: string, error?: string} | null>(null);

  // Use the global shared orderbook service instance instead of creating new ones
  const orderbookServiceRef = useRef(orderbookService);

  const submitOrder = useCallback(async (order: OrderSubmission) => {
    setSubmitting(true);
    setLastResult(null);

    try {
      const result = await orderbookServiceRef.current.submitOrder(order);
      setLastResult(result);
      return result;
    } catch (error) {
      const errorResult = {
        success: false,
        error: error instanceof Error ? error.message : 'Unknown error'
      };
      setLastResult(errorResult);
      return errorResult;
    } finally {
      setSubmitting(false);
    }
  }, []);

  const clearResult = useCallback(() => {
    setLastResult(null);
  }, []);

  return {
    submitOrder,
    submitting,
    lastResult,
    clearResult,
  };
}

/**
 * Hook for checking orderbook service health
 */
export function useOrderbookHealth() {
  const [isHealthy, setIsHealthy] = useState<boolean | null>(null);
  const [checking, setChecking] = useState(false);

  // Use the global shared orderbook service instance instead of creating new ones
  const orderbookServiceRef = useRef(orderbookService);

  const checkHealth = useCallback(async () => {
    setChecking(true);
    try {
      const healthy = await orderbookServiceRef.current.checkHealth();
      setIsHealthy(healthy);
      return healthy;
    } catch (error) {
      setIsHealthy(false);
      return false;
    } finally {
      setChecking(false);
    }
  }, []);

  useEffect(() => {
    checkHealth();

    // Check health every 30 seconds
    const interval = setInterval(checkHealth, 30000);
    return () => clearInterval(interval);
  }, [checkHealth]);

  return {
    isHealthy,
    checking,
    checkHealth,
  };
}