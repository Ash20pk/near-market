export type WalletType = 'near' | 'metamask' | 'walletconnect' | 'phantom' | 'coinbase';

export interface WalletInfo {
  id: string;
  name: string;
  type: WalletType;
  icon: string;
  description: string;
  supportedChains: string[];
  isInstalled?: boolean;
}

export interface WalletConnection {
  type: WalletType;
  account: string;
  chainId?: string;
  provider?: any;
}

export interface WalletState {
  isConnected: boolean;
  connection: WalletConnection | null;
  isConnecting: boolean;
  error: string | null;
}

export interface WalletConnector {
  type: WalletType;
  name: string;
  connect(): Promise<WalletConnection>;
  disconnect(): Promise<void>;
  switchChain?(chainId: string): Promise<void>;
  getBalance?(): Promise<string>;
  signMessage?(message: string): Promise<string>;
  isAvailable(): boolean;
}
