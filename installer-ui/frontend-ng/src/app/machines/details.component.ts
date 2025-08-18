import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ActivatedRoute, Router } from '@angular/router';
import { MachinesService, Machine } from '../services/machines.service';

@Component({
  selector: 'app-machine-details',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './details.component.html',
  styleUrl: './details.component.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class MachineDetailsComponent {
  private readonly route = inject(ActivatedRoute);
  private readonly router = inject(Router);
  private readonly svc = inject(MachinesService);

  readonly id = signal<string>('');

  readonly machines = this.svc.machines;
  readonly loading = this.svc.loading;
  readonly error = this.svc.error;

  readonly machine = computed<Machine | undefined>(() => this.machines().find(m => m.id === this.id()));

  constructor() {
    const paramId = this.route.snapshot.paramMap.get('id') ?? '';
    this.id.set(paramId);
    if (paramId) {
      this.svc.getMachineDetails(paramId).subscribe();
    }
  }

  back() {
    this.router.navigate(['/machines']);
  }
}
