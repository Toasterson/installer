import { Routes } from '@angular/router';

export const routes: Routes = [
  { path: '', pathMatch: 'full', redirectTo: 'machines' },
  {
    path: 'machines',
    loadComponent: () => import('./machines/overview.component').then(m => m.MachinesOverviewComponent)
  },
  {
    path: 'machines/:id',
    loadComponent: () => import('./machines/details.component').then(m => m.MachineDetailsComponent)
  },
  { path: '**', redirectTo: 'machines' }
];
