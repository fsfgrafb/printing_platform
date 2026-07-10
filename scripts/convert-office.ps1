param(
    [Parameter(Mandatory = $true)]
    [string]$InputPath,

    [Parameter(Mandatory = $true)]
    [string]$OutputPath
)

$ErrorActionPreference = "Stop"

$resolvedInputPath = (Resolve-Path -LiteralPath $InputPath).Path
$resolvedOutputPath = [System.IO.Path]::GetFullPath($OutputPath)
$extension = [System.IO.Path]::GetExtension($resolvedInputPath).ToLowerInvariant()
$outputDir = [System.IO.Path]::GetDirectoryName($resolvedOutputPath)

if (-not [System.IO.Directory]::Exists($outputDir)) {
    [System.IO.Directory]::CreateDirectory($outputDir) | Out-Null
}

switch ($extension) {
    ".doc" {
        $word = New-Object -ComObject Word.Application
        $word.Visible = $false
        try {
            $doc = $word.Documents.Open($resolvedInputPath, $false, $true)
            try {
                $doc.ExportAsFixedFormat($resolvedOutputPath, 17)
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
            $doc = $word.Documents.Open($resolvedInputPath, $false, $true)
            try {
                $doc.ExportAsFixedFormat($resolvedOutputPath, 17)
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
            $book = $excel.Workbooks.Open($resolvedInputPath, 3, $true)
            try {
                $book.ExportAsFixedFormat(0, $resolvedOutputPath)
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
            $book = $excel.Workbooks.Open($resolvedInputPath, 3, $true)
            try {
                $book.ExportAsFixedFormat(0, $resolvedOutputPath)
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
            $presentation = $powerPoint.Presentations.Open($resolvedInputPath, $true, $false, $false)
            try {
                $presentation.SaveAs($resolvedOutputPath, 32)
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
            $presentation = $powerPoint.Presentations.Open($resolvedInputPath, $true, $false, $false)
            try {
                $presentation.SaveAs($resolvedOutputPath, 32)
            } finally {
                $presentation.Close()
            }
        } finally {
            $powerPoint.Quit()
        }
    }
    { $_ -in ".jpg", ".jpeg", ".png", ".bmp" } {
        $word = New-Object -ComObject Word.Application
        $word.Visible = $false
        try {
            $doc = $word.Documents.Add()
            try {
                $section = $doc.Sections.Item(1)
                $section.TopMargin = 36
                $section.BottomMargin = 36
                $section.LeftMargin = 36
                $section.RightMargin = 36
                $shape = $doc.InlineShapes.AddPicture($resolvedInputPath)
                $maxWidth = $section.PageSetup.PageWidth - $section.LeftMargin - $section.RightMargin
                $maxHeight = $section.PageSetup.PageHeight - $section.TopMargin - $section.BottomMargin
                if ($shape.Width -gt $maxWidth) { $shape.Width = $maxWidth }
                if ($shape.Height -gt $maxHeight) { $shape.Height = $maxHeight }
                $doc.ExportAsFixedFormat($resolvedOutputPath, 17)
            } finally {
                $doc.Close($false)
            }
        } finally {
            $word.Quit()
        }
    }
    ".txt" {
        $word = New-Object -ComObject Word.Application
        $word.Visible = $false
        try {
            $doc = $word.Documents.Add()
            try {
                $doc.Content.Text = [System.IO.File]::ReadAllText($resolvedInputPath)
                $doc.ExportAsFixedFormat($resolvedOutputPath, 17)
            } finally {
                $doc.Close($false)
            }
        } finally {
            $word.Quit()
        }
    }
    default {
        throw "Unsupported Office file extension: $extension"
    }
}

if (-not [System.IO.File]::Exists($resolvedOutputPath)) {
    throw "Converter did not create output: $resolvedOutputPath"
}
