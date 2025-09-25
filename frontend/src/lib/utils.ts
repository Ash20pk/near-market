import { type ClassValue, clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

export function formatCurrency(
  amount: number | string,
  currency: 'USD' | 'NEAR' = 'USD',
  decimals: number = 2
): string {
  const num = typeof amount === 'string' ? parseFloat(amount) : amount;
  
  if (currency === 'NEAR') {
    return `Ⓝ ${num.toFixed(decimals)}`;
  }
  
  return new Intl.NumberFormat('en-US', {
    style: 'currency',
    currency: 'USD',
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals,
  }).format(num);
}

export function formatPercentage(value: number, decimals: number = 1): string {
  return `${(value * 100).toFixed(decimals)}%`;
}

export function formatTime(timestamp: string | number): string {
  if (!timestamp) {
    return 'Unknown time';
  }

  let date: Date;
  if (typeof timestamp === 'string') {
    const parsed = parseInt(timestamp);
    if (isNaN(parsed)) {
      return 'Invalid time';
    }
    // Assume nanoseconds for NEAR timestamps
    date = new Date(parsed / 1000000);
  } else {
    date = new Date(timestamp);
  }

  // Check if date is valid
  if (isNaN(date.getTime())) {
    return 'Invalid time';
  }

  return date.toLocaleString();
}

export function formatRelativeTime(timestamp: string | number): string {
  try {
    // Handle invalid timestamps
    if (!timestamp) {
      return 'Unknown time';
    }

    // Convert timestamp to Date object
    let date: Date;
    if (typeof timestamp === 'string') {
      const parsed = parseInt(timestamp);
      if (isNaN(parsed)) {
        console.warn('[formatRelativeTime] Invalid timestamp string:', timestamp);
        return 'Invalid time';
      }
      // Assume nanoseconds for NEAR timestamps
      date = new Date(parsed / 1000000);
    } else {
      // Handle numeric timestamps - check if it's likely in nanoseconds
      if (timestamp > 1e12) { // If timestamp is greater than 1 trillion, likely nanoseconds
        date = new Date(timestamp / 1000000);
      } else {
        date = new Date(timestamp);
      }
    }

    // Check if date is valid
    if (isNaN(date.getTime())) {
      console.warn('[formatRelativeTime] Invalid date created from timestamp:', timestamp, 'resulting date:', date);
      return 'Invalid time';
    }

    const now = new Date();
    const diff = date.getTime() - now.getTime();

    // Check if diff is finite
    if (!isFinite(diff)) {
      console.warn('[formatRelativeTime] Non-finite time difference:', diff, 'from timestamp:', timestamp);
      return 'Invalid time';
    }

    const rtf = new Intl.RelativeTimeFormat('en', { numeric: 'auto' });

    const seconds = Math.floor(diff / 1000);
    const minutes = Math.floor(seconds / 60);
    const hours = Math.floor(minutes / 60);
    const days = Math.floor(hours / 24);
    const months = Math.floor(days / 30);
    const years = Math.floor(days / 365);

    // Add validation before calling rtf.format
    // Handle very large future dates
    if (Math.abs(years) >= 1 && isFinite(years) && years > -1000000 && years < 1000000) {
      return rtf.format(years, 'year');
    } else if (Math.abs(months) >= 1 && isFinite(months) && months > -120000 && months < 120000) {
      return rtf.format(months, 'month');
    } else if (Math.abs(days) >= 1 && isFinite(days) && days > -36500 && days < 36500) {
      return rtf.format(days, 'day');
    } else if (Math.abs(hours) >= 1 && isFinite(hours) && hours > -876000 && hours < 876000) {
      return rtf.format(hours, 'hour');
    } else if (Math.abs(minutes) >= 1 && isFinite(minutes) && minutes > -52560000 && minutes < 52560000) {
      return rtf.format(minutes, 'minute');
    } else if (isFinite(seconds) && seconds > -3153600000 && seconds < 3153600000) {
      return rtf.format(seconds, 'second');
    } else {
      // Value is too large/small for Intl.RelativeTimeFormat
      console.warn('[formatRelativeTime] Value too large for RelativeTimeFormat:', {
        timestamp, years, months, days, hours, minutes, seconds
      });

      // Fallback to absolute date for extreme values
      return date.toLocaleDateString();
    }
  } catch (error) {
    console.error('[formatRelativeTime] Error formatting timestamp:', timestamp, error);
    return 'Invalid time';
  }
}

export function truncateAddress(address: string, chars: number = 4): string {
  if (!address) return '';
  if (address.length <= chars * 2 + 3) return address;
  
  return `${address.slice(0, chars)}...${address.slice(-chars)}`;
}

export function generateIntentId(): string {
  return `intent_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
}

export function basisPointsToPercent(bps: number): number {
  return bps / 10000;
}

export function percentToBasisPoints(percent: number): number {
  return Math.round(percent * 10000);
}

export function calculateProbability(yesShares: number, noShares: number): number {
  if (yesShares + noShares === 0) return 0.5;
  return yesShares / (yesShares + noShares);
}

export function getMarketStatusColor(isActive: boolean, endTime: string): string {
  const now = Date.now() * 1000000; // Convert to nanoseconds
  const end = parseInt(endTime);
  
  if (!isActive) return 'text-red-600';
  if (now > end) return 'text-orange-600';
  return 'text-green-600';
}

export function getMarketStatusText(isActive: boolean, endTime: string): string {
  const now = Date.now() * 1000000; // Convert to nanoseconds  
  const end = parseInt(endTime);
  
  if (!isActive) return 'Inactive';
  if (now > end) return 'Closed';
  return 'Active';
}

export function validateMarketForm(data: {
  title: string;
  description: string;
  endTime: string;
  resolutionTime: string;
  category: string;
}) {
  const errors: Record<string, string> = {};
  
  if (!data.title.trim()) {
    errors.title = 'Title is required';
  } else if (data.title.length < 10) {
    errors.title = 'Title must be at least 10 characters';
  }
  
  if (!data.description.trim()) {
    errors.description = 'Description is required';
  } else if (data.description.length < 50) {
    errors.description = 'Description must be at least 50 characters';
  }
  
  if (!data.category.trim()) {
    errors.category = 'Category is required';
  }
  
  const now = new Date();
  const endDate = new Date(data.endTime);
  const resolutionDate = new Date(data.resolutionTime);
  
  if (endDate <= now) {
    errors.endTime = 'End time must be in the future';
  }
  
  if (resolutionDate <= endDate) {
    errors.resolutionTime = 'Resolution time must be after end time';
  }
  
  return {
    isValid: Object.keys(errors).length === 0,
    errors
  };
}

export function parseContractError(error: any): string {
  if (typeof error === 'string') return error;
  
  if (error?.kind?.ExecutionError) {
    return error.kind.ExecutionError;
  }
  
  if (error?.message) {
    return error.message;
  }
  
  if (error?.type === 'FunctionCallError' && error?.kind?.ExecutionError) {
    return error.kind.ExecutionError;
  }
  
  return 'An unexpected error occurred';
}

export const MARKET_CATEGORIES = [
  'Crypto',
  'Sports', 
  'Politics',
  'Entertainment',
  'Technology',
  'Finance',
  'Weather',
  'Other'
] as const;

export type MarketCategory = typeof MARKET_CATEGORIES[number];

export const INTENT_TYPES = {
  BuyShares: 'Buy Shares',
  SellShares: 'Sell Shares', 
  MintComplete: 'Mint Complete Set',
  RedeemWinning: 'Redeem Winning Shares'
} as const;