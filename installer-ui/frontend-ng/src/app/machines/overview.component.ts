import { ChangeDetectionStrategy, Component, effect, inject, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { MachinesService } from '../services/machines.service';
import { Router } from '@angular/router';

@Component({
  selector: 'app-machines-overview',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './overview.component.html',
  styleUrl: './overview.component.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class MachinesOverviewComponent {
  private readonly svc = inject(MachinesService);
  private readonly router = inject(Router);

  // dialog state
  readonly showAdd = signal(false);
  readonly url = signal('');
  readonly claimPassword = signal('');

  // expose service signals to template
  readonly machines = this.svc.machines;
  readonly loading = this.svc.loading;
  readonly error = this.svc.error;
  readonly selectedIds = this.svc.selectedIds;

  constructor() {
    // Initial fetch
    this.svc.listMachines();
  }

  toggleSelected(id: string) {
    this.svc.toggleSelected(id);
  }
  isSelected(id: string) {
    return this.svc.isSelected(id);
  }

  openAdd() {
    this.showAdd.set(true);
  }
  closeAdd() {
    this.showAdd.set(false);
    this.url.set('');
    this.claimPassword.set('');
  }
  claim() {
    const url = this.url().trim();
    const pw = this.claimPassword().trim();
    if (!url || !pw) return;
    this.svc.claimMachine(url, pw).subscribe({
      next: () => this.closeAdd(),
      error: () => {/* error handled in service */}
    });
  }

  openDetails(id: string) {
    this.router.navigate(['/machines', id]);
  }
}
