/**
 * Live Price Display Component
 * Shows real-time market prices that can be easily embedded in existing UI
 */

'use client';

import React from 'react';
import { useMarketPrice } from '../../hooks/useOrderbook';

interface LivePriceDisplayProps {
  marketId: string;
  outcome: number;
  className?: string;
  showChange?: boolean;
  precision?: number;
  fallback?: React.ReactNode;
}

export function LivePriceDisplay({
  marketId,
  outcome,
  className = '',
  showChange = true,
  precision = 1,
  fallback,
}: LivePriceDisplayProps) {
  const { price: yesPrice, loading: yesLoading, error: yesError } = useMarketPrice(marketId, 1);
  const { price: noPrice, loading: noLoading, error: noError } = useMarketPrice(marketId, 0);

  const loading = yesLoading || noLoading;
  const error = yesError || noError;

  if (loading) {
    return (
      <div className={`animate-pulse ${className}`}>
        <div className="h-4 bg-gray-600 rounded w-16"></div>
      </div>
    );
  }

  // For binary markets, ensure complementary pricing
  let displayPrice: { mid: number; bid?: number; ask?: number; last?: number };

  if (outcome === 1) {
    // YES outcome
    if (yesPrice) {
      displayPrice = yesPrice;
    } else if (noPrice) {
      // Calculate YES price from NO price
      displayPrice = {
        mid: 100 - noPrice.mid,
        bid: noPrice.ask ? 100 - noPrice.ask : undefined,
        ask: noPrice.bid ? 100 - noPrice.bid : undefined,
        last: noPrice.last ? 100 - noPrice.last : undefined,
      };
    } else {
      if (fallback) {
        return <>{fallback}</>;
      }
      return (
        <div className={`text-gray-500 ${className}`}>
          <span className="text-sm">Price unavailable</span>
        </div>
      );
    }
  } else {
    // NO outcome - calculate complement of YES price
    if (yesPrice) {
      displayPrice = {
        mid: 100 - yesPrice.mid,
        bid: yesPrice.ask ? 100 - yesPrice.ask : undefined,
        ask: yesPrice.bid ? 100 - yesPrice.bid : undefined,
        last: yesPrice.last ? 100 - yesPrice.last : undefined,
      };
    } else if (noPrice) {
      displayPrice = noPrice;
    } else {
      if (fallback) {
        return <>{fallback}</>;
      }
      return (
        <div className={`text-gray-500 ${className}`}>
          <span className="text-sm">Price unavailable</span>
        </div>
      );
    }
  }

  const formatPrice = (value: number) => `${value.toFixed(precision)}¢`;

  return (
    <div className={`flex items-center gap-2 ${className}`}>
      {/* Current mid price */}
      <span className="font-semibold text-white">
        {formatPrice(displayPrice.mid)}
      </span>

      {/* Bid/Ask spread */}
      {displayPrice.bid && displayPrice.ask && (
        <div className="text-xs text-gray-400">
          <span className="text-green-400">{formatPrice(displayPrice.bid)}</span>
          <span className="mx-1">/</span>
          <span className="text-red-400">{formatPrice(displayPrice.ask)}</span>
        </div>
      )}

      {/* Last price change indicator */}
      {showChange && displayPrice.last && (
        <div className={`text-xs font-medium ${
          displayPrice.last > displayPrice.mid ? 'text-green-400' :
          displayPrice.last < displayPrice.mid ? 'text-red-400' :
          'text-gray-400'
        }`}>
          {displayPrice.last > displayPrice.mid ? '↗' : displayPrice.last < displayPrice.mid ? '↘' : '→'}
        </div>
      )}
    </div>
  );
}

/**
 * Simple price component for minimal display
 */
interface SimplePriceProps {
  marketId: string;
  outcome: number;
  className?: string;
  showCurrency?: boolean;
  fallback?: React.ReactNode;
}

export function SimplePrice({
  marketId,
  outcome,
  className = '',
  showCurrency = true,
  fallback,
}: SimplePriceProps) {
  const { price: yesPrice, loading: yesLoading } = useMarketPrice(marketId, 1);
  const { price: noPrice, loading: noLoading } = useMarketPrice(marketId, 0);

  const loading = yesLoading || noLoading;

  if (loading) {
    if (fallback) {
      return <>{fallback}</>;
    }
    return <span className={`text-gray-500 ${className}`}>--</span>;
  }

  // For binary prediction markets, ensure YES + NO = 100¢
  let displayPrice: number;

  if (outcome === 1) {
    // YES outcome: use YES price directly if available
    if (yesPrice?.mid !== undefined) {
      displayPrice = yesPrice.mid;
    } else if (noPrice?.mid !== undefined) {
      // If no YES price but we have NO price, calculate complement
      displayPrice = 100 - noPrice.mid;
    } else {
      // No price data available
      if (fallback) {
        return <>{fallback}</>;
      }
      return <span className={`text-gray-500 ${className}`}>--</span>;
    }
  } else {
    // NO outcome: calculate complement of YES price for proper binary market pricing
    if (yesPrice?.mid !== undefined) {
      displayPrice = 100 - yesPrice.mid;
    } else if (noPrice?.mid !== undefined) {
      // Fallback to direct NO price if YES price unavailable
      displayPrice = noPrice.mid;
    } else {
      // No price data available
      if (fallback) {
        return <>{fallback}</>;
      }
      return <span className={`text-gray-500 ${className}`}>--</span>;
    }
  }

  return (
    <span className={`font-medium ${className}`}>
      {displayPrice.toFixed(1)}{showCurrency ? '¢' : ''}
    </span>
  );
}

/**
 * Price change indicator component
 */
interface PriceChangeProps {
  marketId: string;
  outcome: number;
  className?: string;
}

export function PriceChange({
  marketId,
  outcome,
  className = '',
}: PriceChangeProps) {
  const { price: yesPrice } = useMarketPrice(marketId, 1);
  const { price: noPrice } = useMarketPrice(marketId, 0);

  // Calculate display price using the same binary market logic
  let displayPrice: { mid: number; last?: number } | null = null;

  if (outcome === 1) {
    // YES outcome
    if (yesPrice) {
      displayPrice = yesPrice;
    } else if (noPrice) {
      displayPrice = {
        mid: 100 - noPrice.mid,
        last: noPrice.last ? 100 - noPrice.last : undefined,
      };
    }
  } else {
    // NO outcome - calculate complement of YES price
    if (yesPrice) {
      displayPrice = {
        mid: 100 - yesPrice.mid,
        last: yesPrice.last ? 100 - yesPrice.last : undefined,
      };
    } else if (noPrice) {
      displayPrice = noPrice;
    }
  }

  if (!displayPrice || !displayPrice.last) {
    return null;
  }

  const change = displayPrice.mid - displayPrice.last;
  const changePercent = (change / displayPrice.last) * 100;

  if (Math.abs(changePercent) < 0.1) {
    return null; // Don't show very small changes
  }

  const isPositive = change > 0;

  return (
    <div className={`flex items-center gap-1 text-xs ${className}`}>
      <span className={isPositive ? 'text-green-400' : 'text-red-400'}>
        {isPositive ? '▲' : '▼'}
      </span>
      <span className={isPositive ? 'text-green-400' : 'text-red-400'}>
        {Math.abs(changePercent).toFixed(1)}%
      </span>
    </div>
  );
}