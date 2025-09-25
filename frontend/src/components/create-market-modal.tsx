'use client';

import React, { useState } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import { Select } from '@/components/ui/select';
import { useWallet } from '@/components/near-wallet';
import { validateMarketForm, MARKET_CATEGORIES } from '@/lib/utils';
import { X, Plus, Calendar } from 'lucide-react';

interface CreateMarketModalProps {
  isOpen: boolean;
  onClose: () => void;
  onMarketCreated?: (marketId: string) => void;
}

export function CreateMarketModal({ isOpen, onClose, onMarketCreated }: CreateMarketModalProps) {
  const { nearService, isSignedIn } = useWallet();
  const [loading, setLoading] = useState(false);
  const [errors, setErrors] = useState<Record<string, string>>({});
  
  const [formData, setFormData] = useState({
    title: '',
    description: '',
    endTime: '',
    resolutionTime: '',
    category: '',
    resolver: 'resolver.testnet' // Default resolver
  });

  const handleInputChange = (field: string, value: string) => {
    setFormData(prev => ({ ...prev, [field]: value }));
    if (errors[field]) {
      setErrors(prev => ({ ...prev, [field]: '' }));
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    
    if (!isSignedIn) {
      alert('Please connect your wallet first');
      return;
    }

    const validation = validateMarketForm(formData);
    if (!validation.isValid) {
      setErrors(validation.errors);
      return;
    }

    setLoading(true);
    
    try {
      // Convert dates to nanosecond timestamps
      const endTimeNs = new Date(formData.endTime).getTime() * 1000000;
      const resolutionTimeNs = new Date(formData.resolutionTime).getTime() * 1000000;
      
      const marketId = await nearService.createMarket({
        title: formData.title,
        description: formData.description,
        endTime: endTimeNs.toString(),
        resolutionTime: resolutionTimeNs.toString(),
        category: formData.category,
        resolver: formData.resolver
      });

      if (marketId) {
        onMarketCreated?.(marketId);
        onClose();
        // Reset form
        setFormData({
          title: '',
          description: '',
          endTime: '',
          resolutionTime: '',
          category: '',
          resolver: 'resolver.testnet'
        });
      } else {
        throw new Error('Failed to create market');
      }
    } catch (error: any) {
      console.error('Error creating market:', error);
      setErrors({ submit: error.message || 'Failed to create market' });
    } finally {
      setLoading(false);
    }
  };

  const getMinDateTime = () => {
    const now = new Date();
    now.setHours(now.getHours() + 1); // Minimum 1 hour from now
    return now.toISOString().slice(0, 16);
  };

  const getMinResolutionTime = () => {
    if (!formData.endTime) return getMinDateTime();
    const endTime = new Date(formData.endTime);
    endTime.setHours(endTime.getHours() + 1); // Minimum 1 hour after end time
    return endTime.toISOString().slice(0, 16);
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center p-4 z-50">
      <Card className="w-full max-w-2xl max-h-[90vh] overflow-y-auto">
        <CardHeader className="flex flex-row items-center justify-between">
          <CardTitle className="text-xl">Create New Market</CardTitle>
          <Button variant="ghost" size="sm" onClick={onClose}>
            <X className="w-4 h-4" />
          </Button>
        </CardHeader>

        <CardContent>
          <form onSubmit={handleSubmit} className="space-y-6">
            {/* Title */}
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Market Title *
              </label>
              <Input
                placeholder="e.g., Will Bitcoin reach $100,000 by end of 2024?"
                value={formData.title}
                onChange={(e) => handleInputChange('title', e.target.value)}
                className={errors.title ? 'border-red-500' : ''}
              />
              {errors.title && (
                <p className="text-red-500 text-xs mt-1">{errors.title}</p>
              )}
            </div>

            {/* Description */}
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Description *
              </label>
              <Textarea
                placeholder="Provide detailed resolution criteria and any relevant information..."
                rows={4}
                value={formData.description}
                onChange={(e) => handleInputChange('description', e.target.value)}
                className={errors.description ? 'border-red-500' : ''}
              />
              {errors.description && (
                <p className="text-red-500 text-xs mt-1">{errors.description}</p>
              )}
            </div>

            {/* Category */}
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Category *
              </label>
              <Select
                value={formData.category}
                onChange={(e) => handleInputChange('category', e.target.value)}
                className={errors.category ? 'border-red-500' : ''}
              >
                <option value="">Select a category</option>
                {MARKET_CATEGORIES.map(category => (
                  <option key={category} value={category}>{category}</option>
                ))}
              </Select>
              {errors.category && (
                <p className="text-red-500 text-xs mt-1">{errors.category}</p>
              )}
            </div>

            {/* End Time */}
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Trading End Time *
              </label>
              <Input
                type="datetime-local"
                value={formData.endTime}
                min={getMinDateTime()}
                onChange={(e) => handleInputChange('endTime', e.target.value)}
                className={errors.endTime ? 'border-red-500' : ''}
              />
              {errors.endTime && (
                <p className="text-red-500 text-xs mt-1">{errors.endTime}</p>
              )}
              <p className="text-xs text-gray-500 mt-1">
                When should trading stop for this market?
              </p>
            </div>

            {/* Resolution Time */}
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Resolution Time *
              </label>
              <Input
                type="datetime-local"
                value={formData.resolutionTime}
                min={getMinResolutionTime()}
                onChange={(e) => handleInputChange('resolutionTime', e.target.value)}
                className={errors.resolutionTime ? 'border-red-500' : ''}
              />
              {errors.resolutionTime && (
                <p className="text-red-500 text-xs mt-1">{errors.resolutionTime}</p>
              )}
              <p className="text-xs text-gray-500 mt-1">
                When can the market be resolved? Must be after trading ends.
              </p>
            </div>

            {/* Resolver */}
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Resolver Account
              </label>
              <Input
                placeholder="resolver.testnet"
                value={formData.resolver}
                onChange={(e) => handleInputChange('resolver', e.target.value)}
              />
              <p className="text-xs text-gray-500 mt-1">
                The account authorized to resolve this market. Default: resolver.testnet
              </p>
            </div>

            {/* Market Creation Cost */}
            <div className="bg-blue-50 border border-blue-200 rounded-lg p-4">
              <div className="flex items-start gap-3">
                <Calendar className="w-5 h-5 text-blue-600 mt-0.5" />
                <div>
                  <div className="font-medium text-blue-900">Market Creation</div>
                  <div className="text-sm text-blue-700 mt-1">
                    Creating a market requires a 1 NEAR deposit for storage and gas fees.
                    This will be deducted from your wallet balance.
                  </div>
                </div>
              </div>
            </div>

            {/* Error Display */}
            {errors.submit && (
              <div className="bg-red-50 border border-red-200 rounded-lg p-3">
                <p className="text-red-700 text-sm">{errors.submit}</p>
              </div>
            )}

            {/* Action Buttons */}
            <div className="flex gap-3 pt-4">
              <Button
                type="button"
                variant="outline"
                onClick={onClose}
                className="flex-1"
              >
                Cancel
              </Button>
              <Button
                type="submit"
                disabled={loading || !isSignedIn}
                loading={loading}
                className="flex-1"
              >
                <Plus className="w-4 h-4 mr-2" />
                Create Market
              </Button>
            </div>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}