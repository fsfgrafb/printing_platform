[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("Status", "Submit", "Cancel")]
    [string]$Action,

    [Parameter(Mandatory = $true)]
    [string]$PrinterName,

    [string]$PdfPath,
    [long]$TaskId,
    [int]$JobId,

    [ValidateRange(1, 120)]
    [int]$DiscoverySeconds = 20,
    [string]$PdfPrinterPath = ""
)

$ErrorActionPreference = "Stop"
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
$OutputEncoding = [Console]::OutputEncoding

function Get-Queue {
    Add-Type -AssemblyName System.Printing
    $server = [System.Printing.LocalPrintServer]::new()
    $queue = $server.GetPrintQueue($PrinterName)
    $queue.Refresh()
    return $queue
}

function Get-Jobs {
    return @(Get-PrintJob -PrinterName $PrinterName -ErrorAction Stop | ForEach-Object {
        [ordered]@{
            id = [long]$_.ID
            name = [string]$_.DocumentName
            status = [string]$_.JobStatus
            total_pages = [long]$_.TotalPages
            pages_printed = [long]$_.PagesPrinted
        }
    })
}

function Write-Result {
    param([Parameter(Mandatory)]$Value)
    [Console]::Out.WriteLine(($Value | ConvertTo-Json -Compress -Depth 6))
}

function Remove-NativePrintJob {
    param([int]$Id)
    if (-not ("PrintServerNativeJob" -as [type])) {
        Add-Type -TypeDefinition @"
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;
public static class PrintServerNativeJob {
    const uint JOB_CONTROL_DELETE = 5;
    [DllImport("winspool.drv", EntryPoint="OpenPrinterW", SetLastError=true, CharSet=CharSet.Unicode)]
    static extern bool OpenPrinter(string name, out IntPtr handle, IntPtr defaults);
    [DllImport("winspool.drv", EntryPoint="SetJobW", SetLastError=true, CharSet=CharSet.Unicode)]
    static extern bool SetJob(IntPtr handle, uint id, uint level, IntPtr job, uint command);
    [DllImport("winspool.drv", SetLastError=true)] static extern bool ClosePrinter(IntPtr handle);
    public static void Delete(string printer, uint id) {
        IntPtr handle;
        if (!OpenPrinter(printer, out handle, IntPtr.Zero)) throw new Win32Exception(Marshal.GetLastWin32Error());
        try {
            if (!SetJob(handle, id, 0, IntPtr.Zero, JOB_CONTROL_DELETE)) throw new Win32Exception(Marshal.GetLastWin32Error());
        } finally { ClosePrinter(handle); }
    }
}
"@ | Out-Null
    }
    [PrintServerNativeJob]::Delete($PrinterName, [uint32]$Id)
}

function Test-JobPresent {
    param([int]$Id)
    return $null -ne (Get-PrintJob -PrinterName $PrinterName -ID $Id -ErrorAction SilentlyContinue)
}

function Find-PdfPrinter {
    $candidates = [System.Collections.Generic.List[string]]::new()
    if (-not [string]::IsNullOrWhiteSpace($PdfPrinterPath)) { $candidates.Add($PdfPrinterPath) }
    $candidates.Add((Join-Path $PSScriptRoot "..\tools\SumatraPDF.exe"))
    if ($env:ProgramFiles) {
        $candidates.Add((Join-Path $env:ProgramFiles "SumatraPDF\SumatraPDF.exe"))
        $candidates.Add((Join-Path $env:ProgramFiles "Adobe\Acrobat DC\Acrobat\Acrobat.exe"))
        $candidates.Add((Join-Path $env:ProgramFiles "Adobe\Acrobat Reader\Reader\AcroRd32.exe"))
    }
    if (${env:ProgramFiles(x86)}) {
        $candidates.Add((Join-Path ${env:ProgramFiles(x86)} "SumatraPDF\SumatraPDF.exe"))
        $candidates.Add((Join-Path ${env:ProgramFiles(x86)} "Adobe\Acrobat Reader DC\Reader\AcroRd32.exe"))
        $candidates.Add((Join-Path ${env:ProgramFiles(x86)} "Adobe\Acrobat Reader\Reader\AcroRd32.exe"))
    }
    if ($env:LOCALAPPDATA) { $candidates.Add((Join-Path $env:LOCALAPPDATA "SumatraPDF\SumatraPDF.exe")) }
    foreach ($candidate in $candidates) {
        if (-not [string]::IsNullOrWhiteSpace($candidate) -and (Test-Path -LiteralPath $candidate -PathType Leaf)) {
            return (Resolve-Path -LiteralPath $candidate).Path
        }
    }
    return $null
}

function Start-PdfPrint {
    param([string]$Path)
    $renderer = Find-PdfPrinter
    if ($renderer) {
        $fileName = [System.IO.Path]::GetFileName($renderer)
        if ($fileName -match "^(AcroRd32|Acrobat)\.exe$") {
            $printer = Get-Printer -Name $PrinterName -ErrorAction Stop
            $arguments = @(
                "/n", "/s", "/o", "/h", "/t",
                ('"{0}"' -f $Path),
                ('"{0}"' -f $PrinterName),
                ('"{0}"' -f $printer.DriverName),
                ('"{0}"' -f $printer.PortName)
            )
            return Start-Process -FilePath $renderer -ArgumentList $arguments -WindowStyle Hidden -PassThru
        }

        $arguments = @(
            "-print-to", ('"{0}"' -f $PrinterName),
            "-print-settings", '"simplex,monochrome,fit"',
            "-silent", ('"{0}"' -f $Path)
        )
        return Start-Process -FilePath $renderer -ArgumentList $arguments -WindowStyle Hidden -PassThru
    }

    $startInfo = [System.Diagnostics.ProcessStartInfo]::new($Path)
    if (@($startInfo.Verbs) -contains "PrintTo") {
        return Start-Process -FilePath $Path -Verb PrintTo -ArgumentList ('"{0}"' -f $PrinterName) -WindowStyle Hidden -PassThru
    }

    throw "No PDF command-line printer is installed and .pdf has no PrintTo association. Install SumatraPDF system-wide, place SumatraPDF.exe in tools, or set printer.pdf_printer_path in config.toml."
}

switch ($Action) {
    "Status" {
        try {
            $queue = Get-Queue
            Write-Result ([ordered]@{
                available = $true
                status = if ($queue.QueueStatus -eq [System.Printing.PrintQueueStatus]::None) { "Normal" } else { $queue.QueueStatus.ToString() }
                is_offline = $queue.IsOffline
                is_paused = $queue.IsPaused
                is_printing = $queue.IsPrinting
                is_processing = $queue.IsProcessing
                is_busy = $queue.IsBusy
                is_in_error = $queue.IsInError
                is_not_available = $queue.IsNotAvailable
                need_user_intervention = $queue.NeedUserIntervention
                is_out_of_paper = $queue.IsOutOfPaper
                has_paper_problem = $queue.HasPaperProblem
                is_paper_jammed = $queue.IsPaperJammed
                is_toner_low = $queue.IsTonerLow
                is_out_of_memory = $queue.IsOutOfMemory
                is_output_bin_full = $queue.IsOutputBinFull
                is_door_opened = $queue.IsDoorOpened
                is_manual_feed_required = $queue.IsManualFeedRequired
                is_initializing = $queue.IsInitializing
                is_waiting = $queue.IsWaiting
                is_server_unknown = $queue.IsServerUnknown
                jobs = @(Get-Jobs)
            })
        } catch {
            $printer = Get-Printer -Name $PrinterName -ErrorAction Stop
            $status = [string]$printer.PrinterStatus
            Write-Result ([ordered]@{
                available = $true
                status = $status
                is_offline = $status -match "Offline"
                is_paused = $status -match "Paused"
                is_in_error = $status -match "Error"
                is_not_available = $status -match "NotAvailable"
                need_user_intervention = $false
                is_out_of_paper = $status -match "PaperOut"
                has_paper_problem = $status -match "PaperProblem"
                is_paper_jammed = $status -match "PaperJam"
                is_toner_low = $status -match "TonerLow|NoToner"
                is_out_of_memory = $status -match "OutOfMemory"
                is_output_bin_full = $status -match "OutputBinFull"
                is_door_opened = $status -match "DoorOpen"
                is_manual_feed_required = $status -match "ManualFeed"
                is_server_unknown = $false
                jobs = @(Get-Jobs)
            })
        }
    }
    "Submit" {
        if ([string]::IsNullOrWhiteSpace($PdfPath)) { throw "PdfPath is required" }
        $resolvedPdf = (Resolve-Path -LiteralPath $PdfPath).Path
        if ([System.IO.Path]::GetExtension($resolvedPdf) -ne ".pdf") { throw "Only PDF files can be submitted" }

        # A unique copy gives the Windows queue a stable document name that can be
        # reconciled after a service restart even if the numeric job id is reused.
        $spoolCopy = Join-Path ([System.IO.Path]::GetDirectoryName($resolvedPdf)) "print-task-$TaskId.pdf"
        Copy-Item -LiteralPath $resolvedPdf -Destination $spoolCopy -Force
        $before = @(Get-Jobs | ForEach-Object { $_.id })

        try {
            $rendererProcess = Start-PdfPrint -Path $spoolCopy
            $deadline = [DateTime]::UtcNow.AddSeconds($DiscoverySeconds)
            do {
                Start-Sleep -Milliseconds 250
                $jobs = @(Get-Jobs)
                $newJobs = @($jobs | Where-Object { $before -notcontains $_.id })
                $job = $newJobs | Where-Object { $_.name -like "*print-task-$TaskId*" } | Select-Object -First 1
                if (-not $job -and $newJobs.Count -eq 1) { $job = $newJobs[0] }
                if ($job) {
                    Write-Result ([ordered]@{ job_id = $job.id; job_name = $job.name })
                    exit 0
                }
                if ($rendererProcess -and $rendererProcess.HasExited -and $rendererProcess.ExitCode -ne 0) {
                    throw "PDF renderer exited with code $($rendererProcess.ExitCode) before creating a Windows print job"
                }
            } while ([DateTime]::UtcNow -lt $deadline)
            throw "The PDF print handler did not create a discoverable Windows print job within $DiscoverySeconds seconds"
        } finally {
            # Shell print handlers read the file asynchronously. Keep the unique copy;
            # normal data retention cleanup removes it together with the task preview.
        }
    }
    "Cancel" {
        if ($JobId -le 0) { throw "JobId is required" }
        if (-not (Test-JobPresent $JobId)) {
            Write-Result ([ordered]@{ cancelled = $true; job_id = $JobId; method = "AlreadyAbsent" })
            exit 0
        }

        $methods = [System.Collections.Generic.List[string]]::new()
        try {
            Remove-PrintJob -PrinterName $PrinterName -ID $JobId -Confirm:$false -ErrorAction Stop
            $methods.Add("Remove-PrintJob")
        } catch { }
        Start-Sleep -Seconds 1

        if (Test-JobPresent $JobId) {
            try {
                $queue = Get-Queue
                $job = $queue.GetJob($JobId)
                $job.Cancel()
                $methods.Add("System.Printing.Cancel")
            } catch { }
            Start-Sleep -Seconds 1
        }

        if (Test-JobPresent $JobId) {
            try {
                $cimJob = Get-CimInstance Win32_PrintJob -ErrorAction Stop | Where-Object { [int]$_.JobId -eq $JobId } | Select-Object -First 1
                if ($cimJob) {
                    Remove-CimInstance -InputObject $cimJob -ErrorAction Stop
                    $methods.Add("Remove-CimInstance")
                }
            } catch { }
            Start-Sleep -Seconds 1
        }

        if (Test-JobPresent $JobId) {
            try {
                Remove-NativePrintJob -Id $JobId
                $methods.Add("SetJob(JOB_CONTROL_DELETE)")
            } catch { }
        }

        $deadline = [DateTime]::UtcNow.AddSeconds(10)
        do {
            Start-Sleep -Milliseconds 500
            if (-not (Test-JobPresent $JobId)) {
                Write-Result ([ordered]@{ cancelled = $true; job_id = $JobId; methods = @($methods) })
                exit 0
            }
        } while ([DateTime]::UtcNow -lt $deadline)
        throw "Windows print job $JobId is still present after cancellation; check the queue or restart the printer"
    }
}
