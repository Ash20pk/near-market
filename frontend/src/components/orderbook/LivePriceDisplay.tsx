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
  const { price, loading, error } = useMarketPrice(marketId, outcome);

  if (loading) {
    return (
      <div className={`animate-pulse ${className}`}>
        <div className="h-4 bg-gray-600 rounded w-16"></div>
      </div>
    );
  }

  if (error || !price) {
    if (fallback) {
      return <>{fallback}</>;
    }
    return (
      <div className={`text-gray-500 ${className}`}>
        <span className="text-sm">Price unavailable</span>
      </div>
    );
  }

  const formatPrice = (value: number) => `${value.toFixed(precision)}¢`;

  return (
    <div className={`flex items-center gap-2 ${className}`}>
      {/* Current mid price */}
      <span className="font-semibold text-white">
        {formatPrice(price.mid)}
      </span>

      {/* Bid/Ask spread */}
      <div className="text-xs text-gray-400">
        <span className="text-green-400">{formatPrice(price.bid)}</span>
        <span className="mx-1">/</span>
        <span className="text-red-400">{formatPrice(price.ask)}</span>
      </div>

      {/* Last price change indicator */}
      {showChange && price.last && (
        <div className={`text-xs font-medium ${
          price.last > price.mid ? 'text-green-400' :
          price.last < price.mid ? 'text-red-400' :
          'text-gray-400'
        }`}>
          {price.last > price.mid ? '↗' : price.last < price.mid ? '↘' : '→'}
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
  const { price, loading } = useMarketPrice(marketId, outcome);

  if (loading || !price) {
    if (fallback) {
      return <>{fallback}</>;
    }
    return <span className={`text-gray-500 ${className}`}>--</span>;
  }

  return (
    <span className={`font-medium ${className}`}>
      {price.mid.toFixed(1)}{showCurrency ? '¢' : ''}
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
  const { price } = useMarketPrice(marketId, outcome);

  if (!price || !price.last) {
    return null;
  }

  const change = price.mid - price.last;
  const changePercent = (change / price.last) * 100;

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