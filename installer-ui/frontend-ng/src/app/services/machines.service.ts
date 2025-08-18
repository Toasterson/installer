import { Injectable, computed, effect, inject, signal } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { toSignal } from '@angular/core/rxjs-interop';
import { map, of, tap } from 'rxjs';

// Minimal local models; replace with OpenAPI-generated types later
export interface SystemInformation {
  hostname?: string;
  osName?: string;
  osVersion?: string;
  kernel?: string;
  arch?: string;
  cpuModel?: string;
  cpuCores?: number;
  memoryTotalBytes?: number;
  memoryFreeBytes?: number;
  uptimeSeconds?: number;
  disks?: Array<{ name?: string; sizeBytes?: number; model?: string; serial?: string }>;
  nics?: Array<{ name?: string; mac?: string; ips?: string[] }>;
}

export interface Machine {
  id: string;
  name?: string;
  url?: string;
  claimed?: boolean;
  system?: SystemInformation;
  createdAt?: string;
  updatedAt?: string;
}

interface ClaimRequest { url: string; claimPassword: string }

@Injectable({ providedIn: 'root' })
export class MachinesService {
  private readonly http = inject(HttpClient);
  private readonly apiBase = '/api';

  private readonly LS_KEY = 'machines-cache-v1';
  private readonly SELECT_KEY = 'machines-selected-v1';

  // state
  readonly machines = signal<Machine[]>(this.readFromLS());
  readonly selectedIds = signal<string[]>(this.readSelectedFromLS());
  readonly loading = signal(false);
  readonly error = signal<string | null>(null);

  constructor() {
    // persist whenever machines change
    effect(() => {
      const current = this.machines();
      try {
        localStorage.setItem(this.LS_KEY, JSON.stringify(current));
      } catch {}
    });
    // persist selection
    effect(() => {
      const selected = this.selectedIds();
      try {
        localStorage.setItem(this.SELECT_KEY, JSON.stringify(selected));
      } catch {}
    });
  }

  listMachines() {
    this.loading.set(true);
    this.error.set(null);
    this.http.get<Machine[]>(`${this.apiBase}/machines`)
      .pipe(
        tap({
          next: (list) => {
            // ensure stable id presence
            const normalized = (list ?? []).filter(Boolean) as Machine[];
            this.machines.set(normalized);
          },
          error: (err) => {
            console.error('listMachines error', err);
            this.error.set('Failed to load machines');
          }
        })
      )
      .subscribe({
        complete: () => this.loading.set(false)
      });
  }

  claimMachine(url: string, claimPassword: string) {
    this.loading.set(true);
    this.error.set(null);
    return this.http.post<Machine>(`${this.apiBase}/machines/claim`, { url, claimPassword } satisfies ClaimRequest)
      .pipe(
        tap({
          next: (m) => {
            // Merge into list immediately
            const existing = this.machines();
            const idx = existing.findIndex(x => x.id === m.id);
            if (idx >= 0) {
              const copy = existing.slice();
              copy[idx] = { ...copy[idx], ...m };
              this.machines.set(copy);
            } else {
              this.machines.set([m, ...existing]);
            }
            // Proactively fetch full details/system info for this machine
            if (m?.id) {
              this.getMachineDetails(m.id).subscribe();
            }
          },
          error: (err) => {
            console.error('claimMachine error', err);
            this.error.set('Failed to claim machine');
          }
        }),
        tap({ complete: () => this.listMachines() })
      );
  }

  getMachineDetails(id: string) {
    this.loading.set(true);
    this.error.set(null);
    return this.http.get<Machine>(`${this.apiBase}/machines/${encodeURIComponent(id)}`)
      .pipe(
        tap({
          next: (m) => {
            // Update machines list cache
            const existing = this.machines();
            const idx = existing.findIndex(x => x.id === id);
            if (idx >= 0) {
              const copy = existing.slice();
              copy[idx] = { ...copy[idx], ...m };
              this.machines.set(copy);
            } else {
              this.machines.set([m, ...existing]);
            }
          },
          error: (err) => {
            console.error('getMachineDetails error', err);
            this.error.set('Failed to load machine details');
          }
        }),
        tap({ complete: () => this.loading.set(false) })
      );
  }

  // selection helpers
  toggleSelected(id: string) {
    const set = new Set(this.selectedIds());
    if (set.has(id)) set.delete(id); else set.add(id);
    this.selectedIds.set(Array.from(set));
  }
  clearSelection() {
    this.selectedIds.set([]);
  }
  setSelection(ids: string[]) {
    const unique = Array.from(new Set(ids));
    this.selectedIds.set(unique);
  }
  isSelected(id: string) {
    return this.selectedIds().includes(id);
  }

  private readFromLS(): Machine[] {
    try {
      const raw = localStorage.getItem(this.LS_KEY);
      if (!raw) return [];
      const parsed = JSON.parse(raw);
      if (Array.isArray(parsed)) return parsed as Machine[];
      return [];
    } catch {
      return [];
    }
  }
  private readSelectedFromLS(): string[] {
    try {
      const raw = localStorage.getItem(this.SELECT_KEY);
      if (!raw) return [];
      const parsed = JSON.parse(raw);
      if (Array.isArray(parsed)) return parsed as string[];
      return [];
    } catch {
      return [];
    }
  }
}
