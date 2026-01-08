# Current Task: Widget Improvements

## Objective
Update Chart, Scatter, Plotter, and Gantt widgets to restrict data to the current day, validate window size, set default settings, and remove redundant settings.

## Todo List

### 1. Default Settings (src/stores/useDashboardStore.ts)
- [x] Update `addWidget` to set default `settings` for `chart`, `scatter`, `plotter`, `gantt`.
  - [x] Chart: `{ timeMode: 'relative', timeWindow: 60, chartType: 'line' }`
  - [x] Plotter: `{ timeMode: 'relative', timeWindow: 60 }`
  - [x] Gantt: `{ timeMode: 'relative', timeWindow: 60 }`
  - [x] Scatter: `{}` (or specific defaults if needed)

### 2. Widget Settings Modal (src/components/dashboard/WidgetSettingsModal.vue)
- [x] Add `min="0"` to "Window Size" input.
- [x] Remove "X-Axis Key" input for `chart` type.
- [x] Change Window Size input to separate Hours, Minutes, Seconds fields.

### 3. Chart Widget (src/components/widgets/ChartWidget.vue)
- [x] Remove `xKey` usage (force time-based X-axis).
- [x] Replace `maxPoints` with `timeWindow` logic.
- [x] Implement "Today Only" filtering (discard data < 00:00:00 today).

### 4. Plotter Widget (src/components/widgets/PlotterWidget.vue)
- [x] Implement "Today Only" filtering in `processMessage`.
- [x] Change X-axis tick label to show time instead of relative seconds.

### 5. Gantt Widget (src/components/widgets/GanttWidget.vue)
- [x] Implement "Today Only" filtering in `processMessage`.
- [x] Change X-axis tick label to show time instead of relative seconds (verify if needed).
- [x] Fix graph display range protruding from X-axis range (enable clipping).

### 6. Scatter Widget (src/components/widgets/ScatterWidget.vue)
- [x] Update internal state to store timestamps.
- [x] Implement "Today Only" filtering.

## Verification
- [x] Verify new widgets have default settings.
- [x] Verify Window Size cannot be negative.
- [x] Verify Chart widget settings do not show X-Axis Key.
- [x] Verify all 4 widgets only show data from the current day.
- [x] Verify Plotter and Gantt show absolute time on X-axis.
- [x] Verify Gantt chart respects X-axis boundaries (clipping).
- [x] Verify Window Size input allows H/M/S setting and updates the total seconds correctly.
