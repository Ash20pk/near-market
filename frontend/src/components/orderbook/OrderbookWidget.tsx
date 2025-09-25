/**
 * Orderbook Widget Component
 * Compact orderbook display that can be embedded in existing UI
 */

'use client';

import React from 'react';
import { useOrderbook } from '../../hooks/useOrderbook';

interface OrderbookWidgetProps {
  marketId: string;
  outcome: number;
  className?: string;
  maxLevels?: number;
  showHeader?: boolean;
}

export function OrderbookWidget({
  marketId,
  outcome,
  className = '',
  maxLevels = 5,
  showHeader = true,
}: OrderbookWidgetProps) {
  const { orderbook, isConnected, error } = useOrderbook(marketId, outcome);

  if (error) {
    return (
      <div className={`text-center text-gray-500 p-4 ${className}`}>
        <div className="text-sm">Orderbook unavailable</div>
      </div>
    );
  }

  if (!orderbook) {
    return (
      <div className={`animate-pulse p-4 ${className}`}>
        <div className="space-y-2">
          {Array(maxLevels).fill(0).map((_, i) => (
            <div key={i} className="flex justify-between">
              <div className="h-3 bg-gray-600 rounded w-12"></div>
              <div className="h-3 bg-gray-600 rounded w-8"></div>
            </div>
          ))}
        </div>
      </div>
    );
  }

  const formatPrice = (price: number) => price.toFixed(1);
  const formatSize = (size: number) => {
    if (size >= 1000) return `${(size / 1000).toFixed(1)}K`;
    return size.toFixed(0);
  };

  const topAsks = orderbook.asks.slice(0, maxLevels).reverse(); // Show highest ask first
  const topBids = orderbook.bids.slice(0, maxLevels);

  return (
    <div className={`bg-gray-900/50 rounded-lg border border-gray-700 ${className}`}>
      {showHeader && (
        <div className="px-3 py-2 border-b border-gray-700 flex items-center justify-between">
          <h3 className="text-sm font-medium text-white">Order Book</h3>
          <div className={`w-2 h-2 rounded-full ${
            isConnected ? 'bg-green-400' : 'bg-red-400'
          }`} />
        </div>
      )}

      <div className="p-3">
        {/* Header */}
        <div className="flex justify-between text-xs text-gray-400 mb-2">
          <span>Price (¢)</span>
          <span>Size</span>
        </div>

        {/* Asks (Sell orders) */}
        <div className="space-y-1 mb-3">
          {topAsks.map((level, index) => (
            <div
              key={`ask-${level.price}`}
              className="flex justify-between text-xs relative"
            >
              {/* Size bar background */}
              <div
                className="absolute right-0 top-0 h-full bg-red-500/10 rounded"
                style={{
                  width: `${Math.min((level.size / Math.max(...orderbook.asks.map(l => l.size))) * 100, 100)}%`
                }}
              />

              <span className="text-red-400 relative z-10">
                {formatPrice(level.price)}
              </span>
              <span className="text-gray-300 relative z-10">
                {formatSize(level.size)}
              </span>
            </div>
          ))}
        </div>

        {/* Spread */}
        {orderbook.asks.length > 0 && orderbook.bids.length > 0 && (
          <div className="text-center border-y border-gray-700 py-1 mb-3">
            <span className="text-xs text-gray-400">
              Spread: {(orderbook.asks[0].price - orderbook.bids[0].price).toFixed(1)}¢
            </span>
          </div>
        )}

        {/* Bids (Buy orders) */}
        <div className="space-y-1">
          {topBids.map((level, index) => (
            <div
              key={`bid-${level.price}`}
              className="flex justify-between text-xs relative"
            >
              {/* Size bar background */}
              <div
                className="absolute right-0 top-0 h-full bg-green-500/10 rounded"
                style={{
                  width: `${Math.min((level.size / Math.max(...orderbook.bids.map(l => l.size))) * 100, 100)}%`
                }}
              />

              <span className="text-green-400 relative z-10">
                {formatPrice(level.price)}
              </span>
              <span className="text-gray-300 relative z-10">
                {formatSize(level.size)}
              </span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

/**
 * Mini orderbook for very compact display
 */
interface MiniOrderbookProps {
  marketId: string;
  outcome: number;
  className?: string;
}

export function MiniOrderbook({
  marketId,
  outcome,
  className = '',
}: MiniOrderbookProps) {
  const { orderbook } = useOrderbook(marketId, outcome);

  if (!orderbook || orderbook.asks.length === 0 || orderbook.bids.length === 0) {
    return (
      <div className={`text-xs text-gray-500 ${className}`}>
        No orders
      </div>
    );
  }

  const bestBid = orderbook.bids[0];
  const bestAsk = orderbook.asks[0];
  const spread = bestAsk.price - bestBid.price;

  return (
    <div className={`flex items-center gap-3 text-xs ${className}`}>
      <div className="text-green-400">
        <span className="font-medium">{bestBid.price.toFixed(1)}¢</span>
        <span className="text-gray-400 ml-1">({bestBid.size.toFixed(0)})</span>
      </div>

      <div className="text-gray-400">
        {spread.toFixed(1)}¢
      </div>

      <div className="text-red-400">
        <span className="font-medium">{bestAsk.price.toFixed(1)}¢</span>
        <span className="text-gray-400 ml-1">({bestAsk.size.toFixed(0)})</span>
      </div>
    </div>
  );
}