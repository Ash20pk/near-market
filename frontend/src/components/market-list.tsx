'use client';

import React, { useState, useEffect } from 'react';
import { Market } from '@/lib/near';
import { useWallet } from '@/components/near-wallet';
import { marketService, EnhancedMarket } from '@/services/market';
import { MarketCard } from '@/components/market-card';
import { Button } from '@/components/ui/button';
import { Select } from '@/components/ui/select';
import { Input } from '@/components/ui/input';
import { Badge } from '@/components/ui/badge';
import { MARKET_CATEGORIES, MarketCategory } from '@/lib/utils';
import { Search, Filter, TrendingUp, Clock, CheckCircle } from 'lucide-react';

interface MarketListProps {
  showTradingButtons?: boolean;
  compact?: boolean;
  limit?: number;
  category?: MarketCategory;
  initialStatus?: 'All' | 'Active' | 'Closed' | 'Resolved';
}

export function MarketList({ 
  showTradingButtons = false, 
  compact = false, 
  limit, 
  category,
  initialStatus
}: MarketListProps) {
  const { nearService, isSignedIn } = useWallet();
  const [markets, setMarkets] = useState<EnhancedMarket[]>([]);
  const [loading, setLoading] = useState(true);
  const [searchTerm, setSearchTerm] = useState('');
  const [selectedCategory, setSelectedCategory] = useState<MarketCategory | 'All'>('All');
  const [statusFilter, setStatusFilter] = useState<'All' | 'Active' | 'Closed' | 'Resolved'>(initialStatus ?? 'All');
  const [sortBy, setSortBy] = useState<'newest' | 'volume' | 'ending'>('newest');

  useEffect(() => {
    loadMarkets();
  }, [selectedCategory, statusFilter]);

  useEffect(() => {
    if (category) {
      setSelectedCategory(category);
    }
  }, [category]);

  const loadMarkets = async () => {
    setLoading(true);
    try {
      const categoryFilter = selectedCategory === 'All' ? undefined : selectedCategory;
      const activeFilter = statusFilter === 'Active' ? true : statusFilter === 'Closed' ? false : undefined;

      // Use the new market service that combines NEAR + orderbook data
      const fetchedMarkets = await marketService.getMarkets(categoryFilter, activeFilter);
      setMarkets(fetchedMarkets);
    } catch (error) {
      console.error('Error loading markets:', error);
      // Fallback handled by market service
      setMarkets([]);
    } finally {
      setLoading(false);
    }
  };


  const filteredAndSortedMarkets = markets
    .filter(market => {
      // Search filter
      if (searchTerm && !market.title.toLowerCase().includes(searchTerm.toLowerCase()) && 
          !market.description.toLowerCase().includes(searchTerm.toLowerCase())) {
        return false;
      }
      return true;
    })
    .sort((a, b) => {
      switch (sortBy) {
        case 'volume':
          return parseInt(b.total_volume || '0') - parseInt(a.total_volume || '0');
        case 'ending':
          return parseInt(a.end_time) - parseInt(b.end_time);
        case 'newest':
        default:
          return parseInt(b.created_at || '0') - parseInt(a.created_at || '0');
      }
    })
    .slice(0, limit);

  if (loading) {
    return (
      <div className="space-y-4">
        {Array.from({ length: 3 }).map((_, i) => (
          <div key={i} className="h-48 bg-gray-200 animate-pulse rounded-lg" />
        ))}
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Filters */}
      <div className="flex flex-col sm:flex-row gap-4 items-start sm:items-center">
        <div className="flex-1 relative">
          <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 text-gray-400 w-4 h-4" />
          <Input
            placeholder="Search markets..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            className="pl-10"
          />
        </div>
        
        <div className="flex gap-2 flex-wrap">
          <Select
            value={selectedCategory}
            onChange={(e) => setSelectedCategory(e.target.value as MarketCategory | 'All')}
          >
            <option value="All">All Categories</option>
            {MARKET_CATEGORIES.map(cat => (
              <option key={cat} value={cat}>{cat}</option>
            ))}
          </Select>
          
          <Select
            value={statusFilter}
            onChange={(e) => setStatusFilter(e.target.value as any)}
          >
            <option value="All">All Status</option>
            <option value="Active">Active</option>
            <option value="Closed">Closed</option>
            <option value="Resolved">Resolved</option>
          </Select>
          
          <Select
            value={sortBy}
            onChange={(e) => setSortBy(e.target.value as any)}
          >
            <option value="newest">Newest</option>
            <option value="volume">Highest Volume</option>
            <option value="ending">Ending Soon</option>
          </Select>
        </div>
      </div>

      {/* Market Stats */}
      <div className="flex gap-4 text-sm text-gray-600">
        <div className="flex items-center gap-1">
          <TrendingUp className="w-4 h-4" />
          {filteredAndSortedMarkets.filter(m => m.is_active).length} Active
        </div>
        <div className="flex items-center gap-1">
          <Clock className="w-4 h-4" />
          {filteredAndSortedMarkets.filter(m => !m.is_active).length} Closed
        </div>
        <div className="flex items-center gap-1">
          <Filter className="w-4 h-4" />
          {filteredAndSortedMarkets.length} Total
        </div>
      </div>

      {/* Markets Grid */}
      {filteredAndSortedMarkets.length === 0 ? (
        <div className="text-center py-12">
          <div className="text-gray-500 mb-4">No markets found</div>
          <Button variant="outline" onClick={loadMarkets}>
            Refresh
          </Button>
        </div>
      ) : (
        <div className={compact ? 'space-y-3' : 'grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-6'}>
          {filteredAndSortedMarkets.map((market) => (
            <MarketCard
              key={market.market_id}
              market={market}
              showTradingButtons={showTradingButtons}
              compact={compact}
            />
          ))}
        </div>
      )}

      {/* Load More */}
      {!limit && filteredAndSortedMarkets.length > 0 && filteredAndSortedMarkets.length % 10 === 0 && (
        <div className="text-center">
          <Button variant="outline" onClick={loadMarkets}>
            Load More Markets
          </Button>
        </div>
      )}
    </div>
  );
}