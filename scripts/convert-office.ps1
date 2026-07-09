param(
    [Parameter(Mandatory = $true)]
    [string]$Input,

    [Parameter(Mandatory = $true)]
    [string]$Output
)

$ErrorActionPreference = "Stop"

$inputPath = (Resolve-Path -LiteralPath $Input).Path
$outputPath = [System.IO.Path]::GetFullPath($Output)
$extension = [System.IO.Path]::GetExtension($inputPath).ToLowerInvariant()
$outputDir = [System.IO.Path]::GetDirectoryName($outputPath)

if (-not [System.IO.Directory]::Exists($outputDir)) {
    [System.IO.Directory]::CreateDirectory($outputDir) | Out-Null
}

switch ($extension) {
    ".doc" {
        $word = New-Object -ComObject Word.Application
        $word.Visible = $false
        try {
            $doc = $word.Documents.Open($inputPath, $false, $true)
            try {
                $doc.ExportAsFixedFormat($outputPath, 17)
            } finally {
                $doc.Close($false)
            }
        } finally {
            $word.Quit()
        }
    }
    ".docx" {
        $word = New-Object -ComObject Word.Application
        $word.Visible = $false
        try {
            $doc = $word.Documents.Open($inputPath, $false, $true)
            try {
                $doc.ExportAsFixedFormat($outputPath, 17)
            } finally {
                $doc.Close($false)
            }
        } finally {
            $word.Quit()
        }
    }
    ".xls" {
        $excel = New-Object -ComObject Excel.Application
        $excel.Visible = $false
        $excel.DisplayAlerts = $false
        try {
            $book = $excel.Workbooks.Open($inputPath, 3, $true)
            try {
                $book.ExportAsFixedFormat(0, $outputPath)
            } finally {
                $book.Close($false)
            }
        } finally {
            $excel.Quit()
        }
    }
    ".xlsx" {
        $excel = New-Object -ComObject Excel.Application
        $excel.Visible = $false
        $excel.DisplayAlerts = $false
        try {
            $book = $excel.Workbooks.Open($inputPath, 3, $true)
            try {
                $book.ExportAsFixedFormat(0, $outputPath)
            } finally {
                $book.Close($false)
            }
        } finally {
            $excel.Quit()
        }
    }
    ".ppt" {
        $powerPoint = New-Object -ComObject PowerPoint.Application
        try {
            $presentation = $powerPoint.Presentations.Open($inputPath, $true, $false, $false)
            try {
                $presentation.SaveAs($outputPath, 32)
            } finally {
                $presentation.Close()
            }
        } finally {
            $powerPoint.Quit()
        }
    }
    ".pptx" {
        $powerPoint = New-Object -ComObject PowerPoint.Application
        try {
            $presentation = $powerPoint.Presentations.Open($inputPath, $true, $false, $false)
            try {
                $presentation.SaveAs($outputPath, 32)
            } finally {
                $presentation.Close()
            }
        } finally {
            $powerPoint.Quit()
        }
    }
    default {
        throw "Unsupported Office file extension: $extension"
    }
}

if (-not [System.IO.File]::Exists($outputPath)) {
    throw "Converter did not create output: $outputPath"
}
