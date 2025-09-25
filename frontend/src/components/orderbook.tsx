"use client";

import React, { useMemo, useState } from "react";

type Side = "bids" | "asks";

interface OrderbookProps {
  marketId: string;
}

interface OrderLevel {
  price: number; // 0-1 dollars for YES share
  size: number; // number of shares
}

export function Orderbook({ marketId }: OrderbookProps) {
  // Mock levels for now – replace with contract data
  const [levels] = useState<{ bids: OrderLevel[]; asks: OrderLevel[] }>(() => ({
    bids: Array.from({ length: 8 }).map((_, i) => ({ price: 0.63 - i * 0.01, size: 50 + i * 10 })),
    asks: Array.from({ length: 8 }).map((_, i) => ({ price: 0.64 + i * 0.01, size: 55 + i * 10 })),
  }));

  const bestBid = levels.bids[0]?.price ?? 0;
  const bestAsk = levels.asks[0]?.price ?? 1;
  const mid = (bestBid + bestAsk) / 2;

  return (
    <div className="bg-black/40 border border-gray-800 rounded-2xl overflow-hidden">
      <div className="p-4 flex items-center justify-between">
        <div>
          <div className="text-xs text-gray-400">Mid</div>
          <div className="text-white font-semibold">{(mid * 100).toFixed(1)}¢</div>
        </div>
        <div className="text-sm text-gray-400">Best Bid {Math.round(bestBid * 100)}¢ • Best Ask {Math.round(bestAsk * 100)}¢</div>
      </div>

      <div className="grid grid-cols-2 divide-x divide-gray-800">
        {/* Asks */}
        <div className="p-3">
          <div className="text-xs text-gray-400 mb-2">Asks (Sell YES)</div>
          <div className="space-y-1">
            {levels.asks.map((lvl, idx) => (
              <div key={`a-${idx}`} className="flex items-center justify-between text-sm bg-red-500/10 hover:bg-red-500/15 rounded-md px-2 py-1">
                <span className="text-red-300">{Math.round(lvl.price * 100)}¢</span>
                <span className="text-gray-300">{lvl.size}</span>
              </div>
            ))}
          </div>
        </div>
        {/* Bids */}
        <div className="p-3">
          <div className="text-xs text-gray-400 mb-2">Bids (Buy YES)</div>
          <div className="space-y-1">
            {levels.bids.map((lvl, idx) => (
              <div key={`b-${idx}`} className="flex items-center justify-between text-sm bg-green-500/10 hover:bg-green-500/15 rounded-md px-2 py-1">
                <span className="text-green-300">{Math.round(lvl.price * 100)}¢</span>
                <span className="text-gray-300">{lvl.size}</span>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
