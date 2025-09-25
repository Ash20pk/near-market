'use client';

// Extend Window interface to include our custom property
declare global {
  interface Window {
    nearInitialized?: boolean;
  }
}

import React, { createContext, useContext, useEffect, useMemo, useRef, useState } from 'react';
// Import wallet selector modal styles
import '@near-wallet-selector/modal-ui/styles.css';
import { truncateAddress, formatCurrency } from '@/lib/utils';
import { Wallet, LogOut, User } from 'lucide-react';
import { initSelector, getSelectorBundle, getActiveAccountId } from '@/lib/near-wallet-selector';
import { nearService } from '@/lib/near';

// Helper: fetch account balance from NEAR RPC and return NEAR units as string
async function fetchNearBalance(network: 'testnet' | 'mainnet', accountId: string): Promise<string> {
  const nodeUrl = network === 'mainnet' ? 'https://rpc.mainnet.near.org' : 'https://rpc.testnet.near.org';
  const body = {
    jsonrpc: '2.0',
    id: 'dontcare',
    method: 'query',
    params: {
      request_type: 'view_account',
      finality: 'final',
      account_id: accountId
    }
  };

  const res = await fetch(nodeUrl, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body)
  });
  if (!res.ok) return '0';
  const data = await res.json();
  const yocto: string | undefined = data?.result?.amount;
  if (!yocto) return '0';
  // Convert yoctoNEAR to NEAR (UI-friendly, not precise accounting)
  const near = Number(yocto.slice(0, -24) || '0') + Number('0.' + yocto.slice(-24).padStart(24, '0'));
  return near.toFixed(2);
}

interface WalletContextType {
  isSignedIn: boolean;
  accountId: string | null;
  balance: string;
  loading: boolean;
  nearService: typeof nearService | null;
  signIn: () => Promise<void>;
  signOut: () => Promise<void>;
}

const WalletContext = createContext<WalletContextType | null>(null);

export function useWallet() {
  const context = useContext(WalletContext);
  if (!context) {
    throw new Error('useWallet must be used within a WalletProvider');
  }
  return context;
}

export function WalletProvider({ children }: { children: React.ReactNode }) {
  const [isSignedIn, setIsSignedIn] = useState(false);
  const [accountId, setAccountId] = useState<string | null>(null);
  const [balance, setBalance] = useState('0');
  const [loading, setLoading] = useState(true);
  const [nearServiceReady, setNearServiceReady] = useState(false);
  const network: 'testnet' | 'mainnet' = (process.env.NEXT_PUBLIC_NEAR_NETWORK as 'testnet' | 'mainnet') || 'testnet';
  const contractId = process.env.NEXT_PUBLIC_VERIFIER_CONTRACT || 'verifier.ashpk20.testnet';

  useEffect(() => {
    initializeWallet();
    }, []);

  const initializeWallet = async () => {
    console.log('[WalletProvider] 🚀 Initializing NEAR Wallet Selector...');
    try {
      // Initialize NearService only once
      if (!window.nearInitialized) {
        console.log('[WalletProvider] ⏳ Initializing NEAR service...');
        await nearService.initialize();
        window.nearInitialized = true;
        setNearServiceReady(true);
        console.log('[WalletProvider] ✅ NEAR service initialized');
      } else {
        setNearServiceReady(true);
        console.log('[WalletProvider] ℹ️ NEAR service already initialized');
      }

      const { selector, modal } = await initSelector(network, contractId);

      // subscribe to account changes
      selector.store.observable.subscribe(async (state) => {
        try {
          const accounts = state.accounts || [];
          const active = accounts.find((a: any) => a.active) || accounts[0];
          const id = active?.accountId || null;
          setAccountId(id);
          setIsSignedIn(!!id);

          if (id) {
            const bal = await fetchNearBalance(network, id);
            setBalance(bal);
          } else {
            setBalance('0');
          }
        } catch (e) {
          console.warn('[WalletProvider] Failed updating account state:', e);
        }
      });

      // Initialize current state
      const id = await getActiveAccountId();
      setIsSignedIn(!!id);
      setAccountId(id);
      if (id) {
        const bal = await fetchNearBalance(network, id);
        setBalance(bal);
      }
    } catch (error) {
      console.error('[WalletProvider] ❌ Failed to initialize Wallet Selector:', error);
    } finally {
      setLoading(false);
    }
  };

  const signIn = async () => {
    const bundle = getSelectorBundle();
    if (!bundle) return;
    bundle.modal.show();
  };

  const signOut = async () => {
    const bundle = getSelectorBundle();
    if (!bundle) return;
    const wallet = await bundle.selector.wallet();
    await wallet.signOut();
  };

  const contextValue: WalletContextType = {
    isSignedIn,
    accountId,
    balance,
    loading,
    nearService: nearServiceReady ? nearService : null,
    signIn,
    signOut
  };

  return (
    <WalletContext.Provider value={contextValue}>
      {children}
    </WalletContext.Provider>
  );
}

export function WalletConnector() {
  const { isSignedIn, accountId, balance, loading, signIn, signOut } = useWallet();

  if (loading) {
    return <div className="h-10 w-32 bg-gray-700 animate-pulse rounded-lg shimmer" />;
  }

  if (!isSignedIn) {
    return (
      <button
        onClick={signIn}
        className="polymarket-button-primary px-4 py-2 text-sm flex items-center gap-2"
      >
        <Wallet className="w-4 h-4" />
        Connect
      </button>
    );
  }

  return (
    <div className="flex items-center gap-3">
      <div className="hidden md:flex items-center gap-3 bg-gray-800/50 rounded-lg px-4 py-2">
        <div className="w-8 h-8 bg-indigo-600/50 rounded-full flex items-center justify-center">
          <User className="w-4 h-4 text-white" />
        </div>
        <div className="text-right text-gray-200">
          <div className="text-sm font-medium">
            {truncateAddress(accountId || '', 6)}
          </div>
        </div>
      </div>
      
      <div className="flex items-center gap-2">
        <button
          onClick={signOut}
          className="polymarket-button-secondary px-3 py-2 text-sm flex items-center gap-2"
        >
          <LogOut className="w-4 h-4" />
          <span className="hidden sm:inline">Disconnect</span>
        </button>
      </div>
    </div>
  );
}