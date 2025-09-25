'use client';

import { setupWalletSelector, WalletSelector, AccountState } from '@near-wallet-selector/core';
import { setupModal, WalletSelectorModal } from '@near-wallet-selector/modal-ui';
import { setupMyNearWallet } from '@near-wallet-selector/my-near-wallet';
import { setupSender } from '@near-wallet-selector/sender';
import { setupMeteorWallet } from '@near-wallet-selector/meteor-wallet';
import { setupLedger } from '@near-wallet-selector/ledger';
import { setupNightly } from '@near-wallet-selector/nightly';
import { setupHereWallet } from '@near-wallet-selector/here-wallet';

export type SelectorBundle = {
  selector: WalletSelector;
  modal: WalletSelectorModal;
};

let selectorBundle: SelectorBundle | null = null;

export async function initSelector(network: 'testnet' | 'mainnet', contractId: string): Promise<SelectorBundle> {
  if (selectorBundle) return selectorBundle;

  const selector = await setupWalletSelector({
    network,
    modules: [
      setupMyNearWallet(),
      setupSender(),
      setupMeteorWallet(),
      setupLedger(),
      setupNightly(),
      setupHereWallet(),
    ],
  });

  const modal = setupModal(selector, {
    contractId,
  });

  selectorBundle = { selector, modal };
  return selectorBundle;
}

export function getSelectorBundle(): SelectorBundle | null {
  return selectorBundle;
}

export async function getActiveAccountId(): Promise<string | null> {
  if (!selectorBundle) return null;
  const state = await selectorBundle.selector.store.getState();
  const accounts = state.accounts as AccountState[];
  const active = accounts.find((a) => a.active) || accounts[0];
  return active?.accountId || null;
}
