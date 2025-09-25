import { WalletConnector, WalletConnection as WalletConnectionType } from '../wallet-types';

declare global {
  interface Window {
    ethereum?: any;
  }
}

export class MetaMaskWalletConnector implements WalletConnector {
  type = 'metamask' as const;
  name = 'MetaMask';

  isAvailable(): boolean {
    return typeof window !== 'undefined' && !!window.ethereum;
  }

  async connect(): Promise<WalletConnectionType> {
    if (typeof window === 'undefined') {
      throw new Error('MetaMask connection is only available in browser');
    }

    if (!window.ethereum) {
      throw new Error('MetaMask not installed');
    }

    try {
      // Request account access
      const accounts = await window.ethereum.request({
        method: 'eth_requestAccounts'
      });

      if (!accounts || accounts.length === 0) {
        throw new Error('No accounts found');
      }

      // Get chain ID
      const chainId = await window.ethereum.request({
        method: 'eth_chainId'
      });

      return {
        type: 'metamask',
        account: accounts[0],
        chainId: chainId,
        provider: window.ethereum
      };
    } catch (error) {
      if (error instanceof Error) {
        throw error;
      }
      throw new Error('Failed to connect to MetaMask');
    }
  }

  async disconnect(): Promise<void> {
    // MetaMask doesn't have a disconnect method
    // We just clear our internal state
    return;
  }

  async switchChain(chainId: string): Promise<void> {
    if (!window.ethereum) {
      throw new Error('MetaMask not available');
    }

    try {
      await window.ethereum.request({
        method: 'wallet_switchEthereumChain',
        params: [{ chainId }],
      });
    } catch (error: any) {
      // If the chain is not added to MetaMask, add it
      if (error.code === 4902) {
        // This would need network parameters to add the chain
        throw new Error('Chain not added to MetaMask. Please add the network manually.');
      }
      throw error;
    }
  }

  async getBalance(): Promise<string> {
    if (!window.ethereum) {
      throw new Error('MetaMask not available');
    }

    // This would need to be implemented based on the specific token contract
    // For now, return a placeholder
    return '0';
  }

  async signMessage(message: string): Promise<string> {
    if (!window.ethereum) {
      throw new Error('MetaMask not available');
    }

    const accounts = await window.ethereum.request({
      method: 'eth_requestAccounts'
    });

    const signature = await window.ethereum.request({
      method: 'personal_sign',
      params: [message, accounts[0]],
    });

    return signature;
  }
}
